// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Program-point variable bindings. Registry transitions select the grammar;
//! this owner follows cell links and joins state along executable CFG edges.

use std::collections::{BTreeMap, HashMap, VecDeque};

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

/// Reaching binding environment before every operation and terminator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointResolveContexts {
    entry: ResolveContext,
    statements: BTreeMap<(BlockId, usize), ResolveContext>,
    terminators: BTreeMap<BlockId, ResolveContext>,
    after: BTreeMap<(BlockId, usize), ResolveContext>,
    reads: BTreeMap<(BlockId, usize), Vec<crate::command_binding::SourceVariableAccess>>,
    source_tokens: BTreeMap<(BlockId, usize), crate::ir::CommandTokens>,
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
            tokens.evaluates_words()
                || matches!(
                    tokens.synthetic,
                    Some(crate::ir::SyntheticMarker::IterationBindings(_))
                )
        })
    }
}

impl PointResolveContexts {
    pub(crate) fn source_token_inventory(
        &self,
    ) -> &BTreeMap<(BlockId, usize), crate::ir::CommandTokens> {
        &self.source_tokens
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
        self.source_tokens = replacements;
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
    };
    retain_source_tokens(&mut result, cfg);
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
            transfer_statement(&mut state, statement, registry);
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

fn retain_source_tokens(result: &mut PointResolveContexts, cfg: &Function) {
    for (&block_id, block) in &cfg.blocks {
        for index in (0..block.statements.len()).chain(std::iter::once(usize::MAX)) {
            if let Some(tokens) = cfg.source_input_tokens_at(block_id, index) {
                if tokens.evaluates_words() {
                    result
                        .reads
                        .insert((block_id, index), tokens.variable_accesses.clone());
                }
                result
                    .source_tokens
                    .insert((block_id, index), tokens.clone());
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
    bind_alias(
        state,
        &VariableCellAliasTransition {
            destination: tcl_registry::VariableAliasDestination::ProcedureLocal,
            local: tcl_registry::TransitionSubject::Literal(local.to_owned()),
            target: VariableAliasTarget::CallerSelectedFrame {
                frame: CallerFrameSelection::Explicit(tcl_registry::TransitionSubject::Literal(
                    "1".to_owned(),
                )),
                variable: tcl_registry::TransitionSubject::Literal(actual.to_owned()),
            },
            writes_value: false,
        },
        registry,
    );
    CallerReferenceBinding::Bound
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
    let target = alias_target(state, &alias.target, registry);
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

fn bind_alias_with_compiled_local(
    state: &mut ResolveContext,
    alias: &VariableCellAliasTransition,
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
            if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS").is_some() {
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
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS").is_some() {
            eprintln!(
                "ORIGINAL_VARIABLE_ALIAS_BIND unknown_destination frame={frame:?} local_ordinal={:?} original_input={} dynamic={}",
                alias.local.argument_index(),
                operands
                    .and_then(|operands| operands.subject_input(&alias.local, state))
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
        || alias_target(state, &alias.target, registry),
        |operands| original_alias_target(state, &alias.target, operands, registry, false),
    );
    #[cfg(test)]
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS").is_some() {
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
    alias: &VariableCellAliasTransition,
    registry: &CommandRegistry,
    original_slot: Option<&Place>,
    key: &crate::var_resolve::VariableCellKey,
    target: &Place,
) -> AliasBindingValidity {
    original_slot.map_or_else(
        || alias_binding_validity(state, alias, registry),
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
    alias: &VariableCellAliasTransition,
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
            || alias_name_target(state, &alias.target, registry),
            |operands| original_alias_target(state, &alias.target, operands, registry, true),
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
    alias: &VariableCellAliasTransition,
    operands: &OriginalVariableInvocation,
    registry: &CommandRegistry,
) -> Place {
    let Some(input) = operands.subject_input(&alias.local, state) else {
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
            if !state.namespace_known && !namespace.starts_with("::") {
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

    if level.is_global_frame() || variable.starts_with("::") {
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
    let context = registry.profile().map(SemanticContext::for_profile);
    let Some(invocation) =
        crate::registry_invocation::resolved_statement_invocation(registry, context, statement)
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
                    .filter(|namespace| state.namespace_known || namespace.starts_with("::"))
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
mod tests {

    fn original_trace_inputs(
        profile: &str,
        prefix: &[u8],
        label: &[u8],
    ) -> (ResolveContext, OriginalVariableInvocation) {
        original_trace_subject_inputs(profile, b"v", prefix, label)
    }

    fn original_trace_subject_inputs(
        profile: &str,
        subject: &[u8],
        prefix: &[u8],
        label: &[u8],
    ) -> (ResolveContext, OriginalVariableInvocation) {
        use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameKey};
        let dialect = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some(profile)).unwrap(),
        );
        let policy = dialect.authored_name_policy().unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let mut source = label.to_vec();
        source.extend_from_slice(b"\ntrace add variable {");
        source.extend_from_slice(subject);
        source.extend_from_slice(b"} read {");
        source.extend_from_slice(prefix);
        source.push(b'}');
        let image = tcl_lexer::SourceImage::native(source);
        let plan = tcl_lexer::native_script_words_in(
            image.clone(),
            tcl_lexer::Span::new(0, u32::try_from(image.len()).unwrap()),
            config,
        )
        .unwrap();
        let inputs = plan
            .commands
            .last()
            .unwrap()
            .words
            .iter()
            .skip(1)
            .map(|word| {
                Some(SignatureSourceNameInput::OriginalWord(
                    SignatureSourceNameKey::from_original_native_word(
                        word,
                        tcl_syntax::word_rules::WordValueRules::from_config(&config),
                        policy,
                    )
                    .unwrap(),
                ))
            })
            .collect();
        let mut state = ResolveContext::for_function("::p");
        state.invocation_dialect = Some(dialect);
        state.execution_name_policy = Some(tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(
            policy,
        ));
        let root = crate::command_binding::SourceNamespaceKey::authored("::");
        state.retain_namespace_world(root.clone(), [root], Some(policy.recipe()));
        (
            state,
            OriginalVariableInvocation::from_original_inputs(inputs, Vec::new()),
        )
    }

    fn selected_variable_trace(profile: &str, remove: bool) -> TraceTransition {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let dialect = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some(profile)).unwrap(),
        );
        let facts = registry
            .resolve_invocation(
                "trace",
                &[
                    if remove { "remove" } else { "add" },
                    "variable",
                    "MISLEADING_TARGET",
                    "read",
                    "MISLEADING_PREFIX",
                ],
                dialect.authoring_query(),
            )
            .unwrap()
            .facts();
        let StateTransition::Trace(trace) =
            &facts.state_transitions.declared().unwrap().facts()[0].transition
        else {
            panic!("selected variable trace");
        };
        trace.clone()
    }

    // Native proof: naming.variable.copied-prefix-storage-removal-report-evaluation
    // docs/design/analysis/name-resolution-proofs/copied-prefix-storage-removal-report-evaluation.md
    #[test]
    fn original_variable_trace_copies_producers_and_removes_by_selected_native_purpose() {
        use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameValue};
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            let (mut state, mut operands) =
                original_trace_inputs(profile, b"watch\0tail extra", b"# first");
            let producer = crate::command_binding::original_name_value::OriginalProducedNameValue::from_source_input(
                operands.input(4, &state).unwrap(), &state,
            ).unwrap();
            operands.inputs[4] = Some(SignatureSourceNameInput::OriginalValue(
                SignatureSourceNameValue::from_original_produced_value(&producer),
            ));
            apply_trace_with_original(
                &mut state,
                &selected_variable_trace(profile, false),
                registry,
                Some(&operands),
            );
            let receiver = operands.access(
                2,
                &state,
                registry,
                tcl_registry::TraceOperation::Read,
                false,
            );
            let key = trace_key(&receiver);
            let registration = &state.trace_registrations.get(&key).unwrap()[0];
            let crate::var_resolve::VariableTracePrefix::Original(prefix) = &registration.1 else {
                panic!("original copied data");
            };
            assert_eq!(prefix.input().bytes(), b"watch\0tail extra");
            assert!(prefix.input().original_word_key().is_none());
            let stored = prefix.clone();
            state.invalidate_original_contents();
            assert!(!producer.is_current(&state));
            assert!(stored.input().is_current(&state));
            assert!(operands.input(4, &state).is_none());
            let (_, short) = original_trace_inputs(profile, b"watch", b"# removal short");
            apply_trace_with_original(
                &mut state,
                &selected_variable_trace(profile, true),
                registry,
                Some(&short),
            );
            assert_eq!(state.trace_registrations.get(&key).unwrap().len(), 1);
            let (_, other_tail) =
                original_trace_inputs(profile, b"watch\0fail extra", b"# removal equal count");
            apply_trace_with_original(
                &mut state,
                &selected_variable_trace(profile, true),
                registry,
                Some(&other_tail),
            );
            assert!(state.trace_registrations.get(&key).unwrap().is_empty());
            assert!(!state.traced.contains(&key));
            assert!(stored.input().is_current(&state));
            let mut missing = state.clone();
            missing.execution_name_policy = None;
            assert!(!stored.input().is_current(&missing));
        }
    }

    #[test]
    // Native proof: naming.variable.trace-subject-counted-zero-address
    // docs/design/analysis/name-resolution-proofs/trace-subject-counted-zero-address.md
    fn original_trace_subject_keeps_cstring_purpose_separate_from_counted_runtime_lookup() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            let (mut state, operands) = original_trace_subject_inputs(
                profile,
                b"v\0tail(k)",
                b"watch\0tail extra",
                b"# subject purpose",
            );
            let subject = operands.trace_subject_access(2, &state, registry);
            let ordinary = operands.access(
                2,
                &state,
                registry,
                tcl_registry::TraceOperation::Read,
                false,
            );
            assert_eq!(subject.cell.as_ref().unwrap().name.as_bytes(), b"v");
            assert!(subject.index.is_none());
            if matches!(profile, "tcl9.0" | "tcl9.1") {
                // The combined object parser cannot see a suffix beyond the
                // raw zero; its retained root remains the whole object.
                assert_eq!(
                    ordinary.cell.as_ref().unwrap().name.as_bytes(),
                    b"v\0tail(k)"
                );
                assert!(ordinary.index.is_none());
                assert_ne!(trace_key(&ordinary), trace_key(&subject));
            }
            apply_trace_with_original(
                &mut state,
                &selected_variable_trace(profile, false),
                registry,
                Some(&operands),
            );
            let key = trace_key(&subject);
            let registrations = state.trace_registrations.get(&key).unwrap();
            assert_eq!(registrations.len(), 1);
            let crate::var_resolve::VariableTracePrefix::Original(prefix) = &registrations[0].1
            else {
                panic!("original copied prefix");
            };
            assert_eq!(prefix.input().bytes(), b"watch\0tail extra");
            assert_eq!(operands.input(2, &state).unwrap().bytes(), b"v\0tail(k)");
        }
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let (state, operands) =
            original_trace_subject_inputs("jim", b"v", b"watch", b"# unavailable");
        assert_eq!(
            operands.trace_subject_access(2, &state, registry).kind,
            PlaceKind::Unknown
        );
    }

    #[test]
    fn original_trace_subject_roles_do_not_read_or_overwrite_value_contents() {
        // Implementation contract: naming.variable.trace-source-receiver-purpose (docs/design/analysis/name-resolution-proofs/trace-source-receiver-purpose.md).
        use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameKey};
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            let (mut state, _) = original_trace_inputs(profile, b"watch", b"# value effects");
            let dialect = state.invocation_dialect.unwrap();
            let policy = dialect.authored_name_policy().unwrap();
            let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
            let value = resolve_literal_access(
                "v",
                &state,
                false,
                registry,
                tcl_registry::TraceOperation::Read,
            );
            state.traced.insert(trace_key(&value));
            assert!(project_access(value, &state, tcl_registry::TraceOperation::Read).observed);
            let mut cases: Vec<(&str, &[&str], bool)> = vec![
                ("trace", &["info", "variable", "v"], false),
                ("trace", &["add", "variable", "v", "read", "watch"], false),
                (
                    "trace",
                    &["remove", "variable", "v", "read", "watch"],
                    false,
                ),
                (
                    "trace",
                    &["add", "variable", "v", "INVALID", "watch"],
                    false,
                ),
                ("set", &["v"], true),
            ];
            if matches!(profile, "tcl8.4" | "tcl8.5" | "tcl8.6") {
                cases.extend([
                    ("trace", &["vinfo", "v"][..], false),
                    ("trace", &["variable", "v", "r", "watch"][..], false),
                    ("trace", &["vdelete", "v", "r", "watch"][..], false),
                ]);
            }
            for (command, words, reads_value) in cases {
                let selected = registry
                    .resolve_invocation(command, words, dialect.authoring_query())
                    .unwrap();
                let facts = selected.facts();
                let arguments =
                    tcl_registry::InvocationArguments::literals(words).with_dialect(dialect);
                let source = format!("{command} {}", words.join(" "));
                let image = tcl_lexer::SourceImage::document(&source);
                let plan = tcl_lexer::native_script_words_in(
                    image.clone(),
                    tcl_lexer::Span::new(0, u32::try_from(image.len()).unwrap()),
                    config,
                )
                .unwrap();
                let inputs = plan.commands[0]
                    .words
                    .iter()
                    .skip(1)
                    .map(|word| {
                        Some(SignatureSourceNameInput::OriginalWord(
                            SignatureSourceNameKey::from_original_native_word(
                                word,
                                tcl_syntax::word_rules::WordValueRules::from_config(&config),
                                policy,
                            )
                            .unwrap(),
                        ))
                    })
                    .collect();
                let operands = OriginalVariableInvocation::from_original_inputs(inputs, Vec::new());
                for inputs in [None, Some(&operands)] {
                    let reads = source_variable_read_places_with_original_operands_impl(
                        &facts, arguments, &state, registry, inputs,
                    );
                    assert_eq!(reads.len(), usize::from(reads_value), "{profile}/{words:?}");
                    assert!(reads.iter().all(|place| place.observed));
                    let writes =
                        source_variable_write_places_with_output_order_and_original_operands_impl(
                            &facts, arguments, &state, registry, None, inputs,
                        );
                    assert!(writes.is_empty(), "{profile}/{words:?}");
                }
                if !reads_value {
                    assert!(facts.arg_roles.iter().any(|&(_, role)| matches!(
                        role,
                        tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarWrite
                    )));
                }
            }
        }
    }

    #[test]
    fn original_variable_trace_join_preserves_all_same_valued_producers() {
        let profile = "tcl8.6";
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let (mut left, left_inputs) = original_trace_inputs(profile, b"watch\0tail", b"# left");
        let (mut right, right_inputs) = original_trace_inputs(profile, b"watch\0tail", b"# right");
        apply_trace_with_original(
            &mut left,
            &selected_variable_trace(profile, false),
            registry,
            Some(&left_inputs),
        );
        apply_trace_with_original(
            &mut right,
            &selected_variable_trace(profile, false),
            registry,
            Some(&right_inputs),
        );
        let key = trace_key(&left_inputs.access(
            2,
            &left,
            registry,
            tcl_registry::TraceOperation::Read,
            false,
        ));
        let first = left.trace_registrations.get(&key).unwrap()[0].1.clone();
        let second = right.trace_registrations.get(&key).unwrap()[0].1.clone();
        assert!(first.same_data(&second));
        assert_ne!(first, second);
        left.join(&right);
        let reaching = left.trace_registrations.get(&key).unwrap();
        assert_eq!(reaching.len(), 1);
        let joined = &reaching[0].1;
        assert_ne!(*joined, first);
        assert_ne!(*joined, second);
        assert_eq!(first.joined(&second).as_ref(), Some(joined));
        assert!(!left.untracked_traces.contains(&key));
    }

    #[test]
    fn original_variable_trace_copy_cannot_reset_the_complete_lineage_bound() {
        use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameValue};
        let (context, operands) = original_trace_inputs("tcl8.6", b"watch", b"# original");
        let mut input = operands.input(4, &context).unwrap().clone();
        let mut copied = 0;
        while let Some(value) =
            SignatureSourceNameValue::copied_variable_trace_prefix(&input, &context)
        {
            input = SignatureSourceNameInput::OriginalValue(value);
            copied += 1;
            assert!(input.original_word_key().is_none());
            assert!(copied < crate::command_binding::original_name_value::MAX_ORIGINS);
        }
        assert_eq!(
            copied + 1,
            crate::command_binding::original_name_value::MAX_ORIGINS
        );
        assert!(input.is_current(&context));
    }

    #[test]
    fn valid_trace_registration_closes_only_the_selected_native_address_protocol() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let mut state = ResolveContext::for_frame(
            crate::var_resolve::VariableFrameKind::Global,
            crate::var_resolve::BindingIdentity::Bound,
            Some(dialect),
        );
        state.namespace_cells = fresh_namespace_cells(registry);
        state.define_literal("scalar", "VALUE", registry);
        state.define_literal("array(k)", "MEMBER", registry);
        for (words, expected) in [
            (["add", "variable", "x", "read", "{"], true),
            (["add", "variable", "x", "", "watch"], false),
            (["add", "variable", "scalar(k)", "read", "watch"], false),
            (["add", "variable", "array", "unset", "watch"], true),
            (["add", "variable", "array(k)", "unset", "watch"], true),
            (["add", "variable", "::missing::x", "read", "watch"], false),
            (["remove", "variable", "scalar(k)", "read", "watch"], true),
            (
                ["remove", "variable", "::missing::x", "read", "watch"],
                true,
            ),
        ] {
            let facts = registry
                .resolve_invocation("trace", &words, dialect.authoring_query())
                .unwrap()
                .facts();
            assert_eq!(
                variable_trace_registration_is_closed(&state, &facts, registry),
                expected,
                "{words:?}"
            );
        }
        let facts = registry
            .resolve_invocation(
                "trace",
                &["add", "variable", "x", "read", "watch", "extra"],
                dialect.authoring_query(),
            )
            .unwrap()
            .facts();
        assert!(!variable_trace_registration_is_closed(
            &state, &facts, registry
        ));
        state.invocation_dialect = None;
        let facts = registry
            .resolve_invocation(
                "trace",
                &["add", "variable", "x", "read", "watch"],
                dialect.authoring_query(),
            )
            .unwrap()
            .facts();
        assert!(!variable_trace_registration_is_closed(
            &state, &facts, registry
        ));
    }

    #[test]
    // Implementation contract: naming.variable.byte-cell-correspondence
    // docs/design/analysis/name-resolution-proofs/variable.byte-cell-correspondence.md
    fn original_alias_operands_ignore_display_and_preserve_runtime_units() {
        use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameKey};
        use tcl_lexer::{LexerConfig, SourceImage, Span};
        use tcl_syntax::naming::{ExecutionNamePolicy, NamePolicyProtocol};
        use tcl_syntax::word_rules::WordValueRules;
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            let dialect = tcl_registry::InvocationDialect::of_point(
                tcl_dialect::model::DialectPoint::of_dialect_name(Some(profile)).unwrap(),
            );
            let policy: NamePolicyProtocol = dialect.authored_name_policy().unwrap();
            let config = LexerConfig::from_grammar(dialect.lexer_grammar);
            for source in [b"v\0tail".as_slice(), br"v\u0000tail", br"v\uD800"] {
                let image = SourceImage::native(source);
                let plan = tcl_lexer::native_script_words_in(
                    image.clone(),
                    Span::new(0, u32::try_from(image.len()).unwrap()),
                    config,
                )
                .unwrap();
                let input = SignatureSourceNameInput::OriginalWord(
                    SignatureSourceNameKey::from_original_native_word(
                        &plan.commands[0].words[0],
                        WordValueRules::from_config(&config),
                        policy,
                    )
                    .unwrap(),
                );
                let mut state = ResolveContext::for_function("::p");
                state.invocation_dialect = Some(dialect);
                state.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(policy));
                let root = crate::command_binding::SourceNamespaceKey::authored("::");
                state.retain_namespace_world(root.clone(), [root.clone()], Some(policy.recipe()));
                // The logical transition facet cannot select either cell.
                let subject = tcl_registry::TransitionSubject::LocatedLiteral {
                    value: "DIFFERENT_DISPLAY".into(),
                    argument_index: 0,
                };
                let alias = VariableCellAliasTransition {
                    destination: tcl_registry::VariableAliasDestination::ProcedureLocal,
                    local: subject.clone(),
                    target: VariableAliasTarget::Global { variable: subject },
                    writes_value: false,
                };
                let operands = OriginalVariableInvocation::from_original_inputs(
                    vec![Some(input.clone())],
                    vec![],
                );
                let expected = crate::var_resolve::resolve_original_namespace_variable_bytes(
                    input.bytes(),
                    &root,
                    &state,
                    registry,
                    false,
                );
                assert_ne!(expected.kind, PlaceKind::Unknown, "{profile}/{source:?}");
                bind_alias_with_compiled_local(&mut state, &alias, registry, None, Some(&operands));
                let local =
                    tcl_syntax::naming::global_local_name_bytes(policy.recipe(), input.bytes())
                        .unwrap();
                let read = crate::var_resolve::resolve_evaluated_variable_input(
                    tcl_syntax::naming::NativeVariableInputForm::Combined(&local),
                    &state,
                    false,
                    registry,
                    tcl_registry::TraceOperation::Read,
                );
                assert_eq!(cell_key(&read), cell_key(&expected), "{profile}/{source:?}");
                assert_eq!(
                    read.cell.as_ref().unwrap().name,
                    expected.cell.as_ref().unwrap().name
                );
            }
            let mut state = ResolveContext::for_function("::p");
            state.invocation_dialect = Some(dialect);
            state.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(policy));
            let subject = tcl_registry::TransitionSubject::LocatedLiteral {
                value: "v".into(),
                argument_index: 0,
            };
            let alias = VariableCellAliasTransition {
                destination: tcl_registry::VariableAliasDestination::ProcedureLocal,
                local: subject.clone(),
                target: VariableAliasTarget::Global { variable: subject },
                writes_value: false,
            };
            let missing = OriginalVariableInvocation::from_original_inputs(vec![None], vec![]);
            bind_alias_with_compiled_local(&mut state, &alias, registry, None, Some(&missing));
            assert!(
                state.dynamic_bindings,
                "{profile}: missing owner must not use displayed v"
            );
        }
    }

    fn original_absolute_alias_operands(
        source: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> OriginalVariableInvocation {
        use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameKey};
        use tcl_lexer::Span;
        use tcl_syntax::word_rules::WordValueRules;
        let parsed = tcl_lexer::native_script_words_in(
            source.clone(),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let inputs = parsed.commands[0].words[1..]
            .iter()
            .map(|word| {
                Some(SignatureSourceNameInput::OriginalWord(
                    SignatureSourceNameKey::from_original_native_word(
                        word,
                        WordValueRules::from_config(&config),
                        policy,
                    )
                    .unwrap(),
                ))
            })
            .collect();
        OriginalVariableInvocation::from_original_inputs(inputs, vec![])
    }

    #[test]
    fn original_absolute_zero_alias_uses_retained_root_without_a_caller_frame() {
        use tcl_lexer::{LexerConfig, SourceImage};
        use tcl_syntax::naming::ExecutionNamePolicy;
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            let dialect = tcl_registry::InvocationDialect::of_point(
                tcl_dialect::model::DialectPoint::of_dialect_name(Some(profile)).unwrap(),
            );
            let policy = dialect.authored_name_policy().unwrap();
            let config = LexerConfig::from_grammar(dialect.lexer_grammar);
            let source = SourceImage::native(br"upvar #0 root\uD800 local".as_slice());
            let operands = original_absolute_alias_operands(&source, config, policy);
            let facts = registry
                .resolve_invocation(
                    "upvar",
                    &["#0", "REPORT_ONLY", "local"],
                    dialect.authoring_query(),
                )
                .unwrap()
                .facts();
            let alias = facts
                .state_transitions
                .declared()
                .unwrap()
                .facts()
                .iter()
                .find_map(|fact| {
                    if let StateTransition::VariableCellAlias(alias) = &fact.transition {
                        Some(alias)
                    } else {
                        None
                    }
                })
                .unwrap();
            let mut state = ResolveContext::for_function("::p");
            state.invocation_dialect = Some(dialect);
            state.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(policy));
            let root = crate::command_binding::SourceNamespaceKey::authored("::");
            state.retain_namespace_world(root.clone(), [root.clone()], Some(policy.recipe()));
            assert!(state.caller.is_none());
            assert!(
                state
                    .selected_frame_context(FrameLevel::Absolute(0))
                    .is_none()
            );
            let expected = crate::var_resolve::resolve_original_namespace_variable_bytes(
                b"root\xed\xa0\x80",
                &root,
                &state,
                registry,
                false,
            );
            let selected = original_alias_target(&state, &alias.target, &operands, registry, false);
            assert_ne!(selected.kind, PlaceKind::Unknown, "{profile}");
            assert_eq!(cell_key(&selected), cell_key(&expected), "{profile}");
            bind_alias_with_compiled_local(&mut state, alias, registry, None, Some(&operands));
            let read = crate::var_resolve::resolve_evaluated_variable_input(
                tcl_syntax::naming::NativeVariableInputForm::Combined(b"local"),
                &state,
                false,
                registry,
                tcl_registry::TraceOperation::Read,
            );
            assert_eq!(cell_key(&read), cell_key(&expected), "{profile}");
            let mut missing = state.clone();
            missing.namespace_identities.remove(&root);
            assert_eq!(
                original_alias_target(&missing, &alias.target, &operands, registry, false).kind,
                PlaceKind::Unknown
            );
            let relative = VariableAliasTarget::CallerSelectedFrame {
                frame: CallerFrameSelection::DefaultCaller,
                variable: tcl_registry::TransitionSubject::LocatedLiteral {
                    value: "REPORT_ONLY".into(),
                    argument_index: 1,
                },
            };
            assert_eq!(
                original_alias_target(&state, &relative, &operands, registry, false).kind,
                PlaceKind::Unknown
            );
        }
    }

    #[test]
    fn normal_alias_completion_requires_the_actual_destination_and_selected_frame() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let mut state = ResolveContext::for_frame(
            crate::var_resolve::VariableFrameKind::Global,
            crate::var_resolve::BindingIdentity::Bound,
            Some(dialect),
        );
        state.namespace_cells = fresh_namespace_cells(registry);
        state.define_literal("b", "100", registry);
        let words = ["0", "b", "link"];
        let arguments = tcl_registry::InvocationArguments::literals(&words).with_dialect(dialect);
        let facts = registry
            .resolve_invocation("upvar", &["0", "b", "link"], dialect.authoring_query())
            .unwrap()
            .facts();
        let before = state.clone();
        assert!(alias_registration_is_closed(
            &state, &facts, arguments, registry
        ));
        state.define_literal("link", "DIRECT", registry);
        assert!(!alias_registration_is_closed(
            &state, &facts, arguments, registry
        ));
        state.contents_world = crate::var_resolve::ContentsWorld::Unknown;
        state.contents_presence.remove("::link");
        assert!(!alias_registration_is_closed(
            &state, &facts, arguments, registry
        ));
        let mut state = before;
        state.caller = None;
        let words = ["1", "b", "link"];
        let arguments = tcl_registry::InvocationArguments::literals(&words).with_dialect(dialect);
        let facts = registry
            .resolve_invocation("upvar", &["1", "b", "link"], dialect.authoring_query())
            .unwrap()
            .facts();
        assert!(!alias_registration_is_closed(
            &state, &facts, arguments, registry
        ));
    }

    #[test]
    fn reached_iteration_assignments_replace_missing_incoming_provenance() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let mut state = ResolveContext::for_function("::p");
        state.invocation_dialect = Some(dialect);
        state.define_literal("unrelated", "KEEP", registry);
        let words = [tcl_registry::InvocationWord::Literal("x y")];
        let arguments = tcl_registry::InvocationArguments::structured(&words).with_dialect(dialect);
        assert_eq!(
            transfer_source_iteration_bindings(&mut state, &[0], arguments, registry, 77),
            SourceIterationBindingOutcome::Entered
        );
        for name in ["x", "y"] {
            let target = resolve_literal_access(
                name,
                &state,
                false,
                registry,
                tcl_registry::TraceOperation::Read,
            );
            assert_eq!(
                state.contents_origin(&target),
                crate::var_resolve::ContentsOrigin::WrittenAt(77)
            );
            assert_eq!(
                state.contents_presence(&target),
                crate::var_resolve::ContentsPresence::Defined
            );
            assert_eq!(state.literal_value(name, registry), None);
        }
        assert_eq!(state.literal_value("unrelated", registry), Some("KEEP"));
    }

    #[test]
    fn iteration_kind_error_preserves_prior_stores_and_selected_container_policy() {
        // Actual C 8.4–9.1 writes x before failing on array a. Jim 0.84
        // replaces the dictionary-valued root and enters the body.
        for (profile, dialect, expected) in [
            (
                "tcl8.6",
                tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
                SourceIterationBindingOutcome::StoreError,
            ),
            (
                "jim",
                tcl_registry::InvocationDialect::of_point(
                    tcl_dialect::model::DialectPoint::canonical(
                        tcl_dialect::model::Release::JIM_0_84,
                    ),
                ),
                SourceIterationBindingOutcome::Entered,
            ),
        ] {
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            let mut state = ResolveContext::for_function("::p");
            state.invocation_dialect = Some(dialect);
            state.define_literal("a(k)", "OLD", registry);
            let element = resolve_literal_access(
                "a(k)",
                &state,
                false,
                registry,
                tcl_registry::TraceOperation::Write,
            );
            state.record_contents_write(&element, 1, false);
            let words = [tcl_registry::InvocationWord::Literal("x a")];
            let arguments =
                tcl_registry::InvocationArguments::structured(&words).with_dialect(dialect);
            assert_eq!(
                transfer_source_iteration_bindings(&mut state, &[0], arguments, registry, 77),
                expected,
                "{profile}"
            );
            let first = resolve_literal_access(
                "x",
                &state,
                false,
                registry,
                tcl_registry::TraceOperation::Read,
            );
            assert_eq!(
                state.contents_origin(&first),
                crate::var_resolve::ContentsOrigin::WrittenAt(77),
                "{profile}"
            );
        }
    }

    #[test]
    fn native_loop_conditions_retain_source_reads_without_replay_boundaries() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for loop_script in [
            "while {$x} {incr x -1}",
            "for {set i 0} {$x} {incr x -1} {}",
        ] {
            let source = format!("proc p {{}} {{set x 1; {loop_script}}}");
            let unit =
                crate::compilation_unit::CompilationUnit::build_for(&source, registry, false);
            let function = unit.function("::p").unwrap();
            let conditions: Vec<_> = function
                .cfg
                .blocks
                .iter()
                .filter_map(|(&block, body)| {
                    let Some(crate::cfg::Terminator::Branch {
                        condition,
                        condition_base,
                        ..
                    }) = &body.terminator
                    else {
                        return None;
                    };
                    matches!(condition, crate::expr_ast::ExprNode::Var { .. }).then_some((
                        block,
                        condition,
                        *condition_base,
                    ))
                })
                .collect();
            assert_eq!(conditions.len(), 1, "{loop_script}");
            for (block, condition, base) in conditions {
                assert!(
                    function.cfg.condition_binding_sites.contains_key(&block),
                    "{loop_script}"
                );
                assert!(
                    !function.cfg.command_boundary_sites.contains_key(&block),
                    "{loop_script}"
                );
                let view = crate::ssa::SsaSourceView::at_terminator(&function.ssa, block);
                assert!(view.source_tokens().is_some(), "{loop_script}");
                assert!(
                    view.read_expression_variable_contents(condition, base, registry)
                        .is_some(),
                    "{loop_script}"
                );
            }
        }
    }
    use super::*;
    use tcl_registry::TransitionSubject;

    #[test]
    fn ordered_output_addresses_decline_only_after_possible_observers() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let words = [
            tcl_registry::InvocationWord::Literal("error fail"),
            tcl_registry::InvocationWord::Literal("result"),
            tcl_registry::InvocationWord::Literal("options"),
        ];
        let arguments = tcl_registry::InvocationArguments::structured(&words);
        let facts = registry
            .resolve_structured_invocation(
                tcl_registry::InvocationWords::from_arguments(
                    tcl_registry::InvocationWord::Literal("catch"),
                    arguments,
                ),
                registry.own_surface_query(),
            )
            .resolved()
            .unwrap()
            .facts();
        let mut state = ResolveContext::for_function("::p");
        state.define_literal("result", "OLD", registry);
        state.define_literal("options", "OLD", registry);
        let result = variable_operand_access(
            arguments,
            1,
            &state,
            registry,
            tcl_registry::TraceOperation::Write,
            false,
        );
        state.traced.insert(cell_key(&result));
        assert_eq!(
            uncertain_variable_output_addresses(&facts, arguments, &state, registry, Some(&[1, 2])),
            std::collections::BTreeSet::from([2])
        );
        assert!(
            uncertain_variable_output_addresses(&facts, arguments, &state, registry, Some(&[2, 1]))
                .is_empty()
        );
        assert_eq!(
            uncertain_variable_output_addresses(&facts, arguments, &state, registry, None),
            std::collections::BTreeSet::from([2])
        );
        let writes = source_variable_write_places_with_output_order(
            &facts,
            arguments,
            &state,
            registry,
            Some(&[1, 2]),
        );
        assert!(writes.iter().any(|place| place.kind == PlaceKind::Unknown));
        assert!(!writes.iter().any(|place| place.name == "options"));
        let options = variable_operand_access(
            arguments,
            2,
            &state,
            registry,
            tcl_registry::TraceOperation::Write,
            false,
        );
        state.traced.insert(cell_key(&options));
        assert_eq!(
            uncertain_variable_output_addresses(&facts, arguments, &state, registry, None),
            std::collections::BTreeSet::from([1, 2])
        );
    }

    #[test]
    fn bounded_array_store_preserves_unrelated_namespace_and_value_facts() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let words = [
            tcl_registry::InvocationWord::ArrayElementName { root: "a" },
            tcl_registry::InvocationWord::Literal("NEW"),
        ];
        let arguments = tcl_registry::InvocationArguments::structured(&words);
        let facts = registry
            .resolve_structured_invocation(
                tcl_registry::InvocationWords::from_arguments(
                    tcl_registry::InvocationWord::Literal("set"),
                    arguments,
                ),
                registry.own_surface_query(),
            )
            .resolved()
            .expect("native set")
            .facts();
        let mut state = ResolveContext::for_function("::top");
        state.define_literal("unrelated", "KEPT", registry);
        let writes = source_variable_write_places(&facts, arguments, &state, registry);
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].kind, PlaceKind::ArrayElem);
        assert_eq!(writes[0].name, "a");
        transfer_source_namespace_cells(&mut state, &facts, arguments, registry, false, 10);
        assert_eq!(state.literal_value("unrelated", registry), Some("KEPT"));
        assert!(state.namespace_cells.present.contains("::a"));
        assert!(!state.dynamic_bindings);
    }

    #[test]
    fn caller_reference_distinguishes_defined_unknown_contents_from_absence() {
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let mut parent = ResolveContext::for_function("::parent");
        parent.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        ));
        parent.define_unknown_contents("parameter", registry);
        let frame = crate::var_resolve::VariableExecutionFrame::Procedure {
            namespace: "::".to_owned(),
            identity: "reference-call".to_owned(),
        };
        let mut child = parent.enter_called_frame(&frame);
        assert_eq!(
            bind_caller_reference(&mut child, "v", Some("parameter"), registry),
            CallerReferenceBinding::Bound
        );
        assert_eq!(
            crate::var_resolve::canonical_literal_variable_name("v", &child, registry),
            crate::var_resolve::canonical_literal_variable_name("parameter", &parent, registry)
        );
        assert_eq!(child.literal_value("v", registry), None);
        let mut child = parent.enter_called_frame(&frame);
        assert_eq!(
            bind_caller_reference(&mut child, "v", Some("absent"), registry),
            CallerReferenceBinding::Missing
        );
        parent.widen();
        let mut child = parent.enter_called_frame(&frame);
        assert_eq!(
            bind_caller_reference(&mut child, "v", Some("absent"), registry),
            CallerReferenceBinding::Unknown
        );
    }

    fn link(state: &mut ResolveContext, target: &str, local: &str) {
        bind_alias(
            state,
            &VariableCellAliasTransition {
                destination: tcl_registry::VariableAliasDestination::CurrentNamespaceOrLocal,
                local: TransitionSubject::Literal(local.to_owned()),
                target: VariableAliasTarget::CallerSelectedFrame {
                    frame: CallerFrameSelection::Explicit(TransitionSubject::Literal(
                        "0".to_owned(),
                    )),
                    variable: TransitionSubject::Literal(target.to_owned()),
                },
                writes_value: false,
            },
            &CommandRegistry::build_default(),
        );
    }

    #[test]
    fn actual_caller_increment_preserves_the_shared_namespace_contents() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut root = ResolveContext::for_function("::top");
        root.invocation_dialect = Some(tcl_registry::InvocationDialect::for_version(
            tcl_dialect::TclVersion::V8_6,
        ));
        root.namespace_cells = fresh_namespace_cells(registry);
        let set_words = [
            tcl_registry::InvocationWord::Literal("n"),
            tcl_registry::InvocationWord::Literal("1"),
        ];
        let set_args = tcl_registry::InvocationArguments::structured(&set_words)
            .with_dialect(root.invocation_dialect.unwrap());
        let set_facts = registry
            .resolve_invocation("set", &["n", "1"], registry.own_surface_query())
            .unwrap()
            .facts();
        transfer_source_namespace_cells(&mut root, &set_facts, set_args, registry, false, 0);
        assert_eq!(root.literal_value("n", registry), Some("1"));
        assert!(
            root.namespace_cells.present.contains("::n"),
            "{set_facts:#?}: {root:#?}"
        );
        let mut child =
            root.enter_called_frame(&crate::var_resolve::VariableExecutionFrame::Procedure {
                namespace: "::".to_owned(),
                identity: "bump@1".to_owned(),
            });
        bind_alias(
            &mut child,
            &VariableCellAliasTransition {
                destination: tcl_registry::VariableAliasDestination::CurrentNamespaceOrLocal,
                local: TransitionSubject::Literal("v".to_owned()),
                target: VariableAliasTarget::CallerSelectedFrame {
                    frame: CallerFrameSelection::Explicit(TransitionSubject::Literal(
                        "1".to_owned(),
                    )),
                    variable: TransitionSubject::Literal("n".to_owned()),
                },
                writes_value: false,
            },
            registry,
        );
        let incr_words = [tcl_registry::InvocationWord::Literal("v")];
        let incr_args = tcl_registry::InvocationArguments::structured(&incr_words)
            .with_dialect(child.invocation_dialect.unwrap());
        let incr_facts = registry
            .resolve_invocation("incr", &["v"], registry.own_surface_query())
            .unwrap()
            .facts();
        transfer_source_namespace_cells(&mut child, &incr_facts, incr_args, registry, false, 10);
        assert_eq!(
            child.literal_value("v", registry),
            Some("2"),
            "{incr_facts:#?}: {child:#?}"
        );
        assert!(
            child.namespace_cells.present.contains("::n"),
            "{incr_facts:#?}: {child:#?}"
        );
        let restored = crate::var_resolve::restore_execution_frame(&root, &child);
        assert_eq!(restored.literal_value("n", registry), Some("2"));
        assert!(restored.namespace_cells.present.contains("::n"));
    }

    #[test]
    fn alias_chain_retargeting_follows_the_runtime_link_policy() {
        for (dialect, expected) in [
            ("tcl8.4", "OLD"),
            ("tcl8.5", "OLD"),
            ("tcl8.6", "OLD"),
            ("tcl9.0", "OLD"),
            ("tcl9.1", "OLD"),
            ("jim", "NEW"),
        ] {
            let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
            let mut state = ResolveContext::for_function("::p");
            state.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
                tcl_registry::model::ingress::resolve_environment(dialect).unit_profile(),
            ));
            state.define_literal("x", "OLD", registry);
            state.define_literal("z", "NEW", registry);
            link(&mut state, "x", "a");
            link(&mut state, "a", "b");
            link(&mut state, "z", "a");
            assert_eq!(
                state.literal_value("b", registry),
                Some(expected),
                "{dialect}: {state:#?}"
            );
        }
    }

    #[test]
    fn namespace_alias_destinations_cannot_retain_c_procedure_cells() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let mut state = ResolveContext::for_function("::p");
        state.known_namespaces.insert("::N".to_owned());
        state.namespace_cells.closed = true;
        let alias = VariableCellAliasTransition {
            destination: tcl_registry::VariableAliasDestination::CurrentNamespaceOrLocal,
            local: TransitionSubject::Literal("::N::y".to_owned()),
            target: VariableAliasTarget::CallerSelectedFrame {
                frame: CallerFrameSelection::Explicit(TransitionSubject::Literal("0".to_owned())),
                variable: TransitionSubject::Literal("x".to_owned()),
            },
            writes_value: false,
        };
        assert_eq!(
            alias_binding_validity(&state, &alias, registry),
            AliasBindingValidity::NamespaceToLocal
        );
        bind_alias(&mut state, &alias, registry);
        assert!(!state.namespace_cells.present.contains("::N::y"));
        assert!(!state.namespace_alias_bindings.contains_key("::N::y"));
    }

    #[test]
    fn namespace_deletion_turns_over_scalar_lifetimes_and_scopes_clobbers() {
        let registry = CommandRegistry::build_default();
        let mut state = ResolveContext::for_function("::p");
        state.known_namespaces.insert("::N".to_owned());
        state.namespace_cells.present.insert("::N::x".to_owned());
        let before = resolve_place("::N::x", &state, false, &registry);
        apply_namespace(
            &mut state,
            &NamespaceTransition::Delete {
                namespace: NamespaceTransitionTarget::Named(TransitionSubject::Literal(
                    "::N".to_owned(),
                )),
            },
            20,
        );
        let after = resolve_place("::N::x", &state, false, &registry);
        assert_ne!(
            crate::var_resolve::canonical_place_name(&before),
            crate::var_resolve::canonical_place_name(&after)
        );
        assert_eq!(
            after.cell.as_ref().map(|cell| cell.generation),
            Some(CellGeneration::After(20))
        );
        let destruction = place::unknown_namespace("::N");
        assert!(place::overlap(&destruction, &before));
        assert!(!place::overlap(
            &destruction,
            &resolve_place("local", &state, false, &registry)
        ));
        assert!(!place::overlap(
            &destruction,
            &resolve_place("::Other::x", &state, false, &registry)
        ));
    }

    #[test]
    fn current_frame_alias_retargets_at_the_binding_operation() {
        let registry = CommandRegistry::build_default();
        let mut state = ResolveContext::for_function("::f");
        link(&mut state, "x", "y");
        let first = resolve_place("y", &state, false, &registry);
        assert_eq!(first, resolve_place("x", &state, false, &registry));
        link(&mut state, "z", "y");
        let second = resolve_place("y", &state, false, &registry);
        assert_eq!(second, resolve_place("z", &state, false, &registry));
        assert!(!place::overlap(&first, &second));
    }

    #[test]
    fn scalar_unset_preserves_alias_but_array_unset_dangles_element_link() {
        let registry = CommandRegistry::build_default();
        let mut state = ResolveContext::for_function("::f");
        link(&mut state, "x", "y");
        destroy_root(&mut state, "y", 10, &registry);
        assert_eq!(
            resolve_place("y", &state, false, &registry),
            resolve_place("x", &state, false, &registry)
        );
        link(&mut state, "a(k)", "element");
        let original = resolve_place("element", &state, false, &registry);
        destroy_root(&mut state, "a", 20, &registry);
        assert!(state.unknown_bindings.contains("element"));
        assert!(!place::overlap(
            &original,
            &resolve_place("a(k)", &state, false, &registry)
        ));
    }

    #[test]
    fn conditional_binding_join_cannot_claim_either_target() {
        let registry = CommandRegistry::build_default();
        let mut left = ResolveContext::for_function("::f");
        let mut right = left.clone();
        link(&mut left, "x", "y");
        link(&mut right, "z", "y");
        left.join(&right);
        assert!(left.unknown_bindings.contains("y"));
        assert!(crate::var_resolve::canonical_variable_name("y", &left, &registry).is_none());
    }
    #[test]
    fn exact_trace_removal_counts_duplicates_and_tracks_cells_across_alias_retargeting() {
        let registry = CommandRegistry::build_default();
        let mut state = ResolveContext::for_function("::f");
        link(&mut state, "x", "y");
        let add = TraceTransition::Add {
            target: TraceTarget::Variable(TransitionSubject::Literal("y".to_owned())),
            operations: tcl_registry::TraceOperationSet::Known(vec![
                tcl_registry::TraceOperation::Read,
            ]),
            prefix: TransitionSubject::Literal("callback".to_owned()),
        };
        apply_trace(&mut state, &add, &registry);
        apply_trace(&mut state, &add, &registry);
        link(&mut state, "z", "y");
        assert!(!resolve_place("y", &state, false, &registry).observed);
        assert!(resolve_place("x", &state, false, &registry).observed);
        let remove = TraceTransition::Remove {
            target: TraceTarget::Variable(TransitionSubject::Literal("x".to_owned())),
            operations: tcl_registry::TraceOperationSet::Known(vec![
                tcl_registry::TraceOperation::Read,
            ]),
            prefix: TransitionSubject::Literal("callback".to_owned()),
        };
        apply_trace(&mut state, &remove, &registry);
        assert!(resolve_place("x", &state, false, &registry).observed);
        apply_trace(&mut state, &remove, &registry);
        assert!(!resolve_place("x", &state, false, &registry).observed);
    }
    fn trace(state: &mut ResolveContext, variable: &str, operation: tcl_registry::TraceOperation) {
        apply_trace(
            state,
            &TraceTransition::Add {
                target: TraceTarget::Variable(TransitionSubject::Literal(variable.to_owned())),
                operations: tcl_registry::TraceOperationSet::Known(vec![operation]),
                prefix: TransitionSubject::Literal("callback".to_owned()),
            },
            &CommandRegistry::build_default(),
        );
    }

    #[test]
    fn trace_callbacks_follow_operation_and_exact_element_address() {
        use tcl_registry::TraceOperation::{Read, Unset, Write};
        let registry = CommandRegistry::build_default();
        let mut state = ResolveContext::for_function("::f");
        trace(&mut state, "a(k)", Write);
        assert!(resolve_literal_access("a(k)", &state, false, &registry, Write).observed);
        assert!(!resolve_literal_access("a(j)", &state, false, &registry, Write).observed);
        assert!(!resolve_literal_access("a(k)", &state, false, &registry, Read).observed);
        assert!(!resolve_literal_access("a", &state, true, &registry, Unset).observed);
        assert!(resolve_literal_access("a", &state, true, &registry, Write).observed);
        trace(&mut state, "a", Read);
        assert!(resolve_literal_access("a(j)", &state, false, &registry, Read).observed);
        assert!(!resolve_literal_access("unrelated", &state, false, &registry, Write).observed);
        trace(&mut state, "a(j)", Unset);
        assert!(resolve_literal_access("a", &state, false, &registry, Unset).observed);
    }

    #[test]
    fn braced_substitution_preserves_literal_element_and_dollar_name() {
        use tcl_registry::TraceOperation::Read;
        let registry = CommandRegistry::build_default();
        let mut state = ResolveContext::for_function("::f");
        trace(&mut state, "a($i)", Read);
        trace(&mut state, "$x", Read);
        let literal =
            crate::var_resolve::resolve_substitution_access("${a($i)}", &state, &registry, Read);
        assert_eq!(
            literal.index.as_ref().map(|index| index.kind),
            Some(place::IndexKind::Literal)
        );
        assert!(literal.observed);
        assert!(!resolve_literal_access("a(k)", &state, false, &registry, Read).observed);
        let dynamic =
            crate::var_resolve::resolve_substitution_access("$a($i)", &state, &registry, Read);
        assert_eq!(
            dynamic.index.as_ref().map(|index| index.kind),
            Some(place::IndexKind::Dynamic)
        );
        assert!(dynamic.observed);
        assert!(
            crate::var_resolve::resolve_substitution_access("${$x}", &state, &registry, Read)
                .observed
        );
    }

    #[test]
    fn unset_removes_element_trace_before_recreation() {
        use tcl_registry::TraceOperation::Write;
        let registry = CommandRegistry::build_default();
        let mut state = ResolveContext::for_function("::f");
        trace(&mut state, "a(k)", Write);
        destroy_root(&mut state, "a(k)", 10, &registry);
        assert!(!resolve_literal_access("a(k)", &state, false, &registry, Write).observed);
        trace(&mut state, "a(k)", Write);
        destroy_root(&mut state, "a", 20, &registry);
        assert!(!resolve_literal_access("a(k)", &state, false, &registry, Write).observed);
    }

    #[test]
    fn namespace_deletion_removes_trace_registrations_from_old_cells() {
        use tcl_registry::TraceOperation::Write;
        let registry = CommandRegistry::build_default();
        let mut state = ResolveContext::for_function("::f");
        state.known_namespaces.insert("::N".to_owned());
        trace(&mut state, "::N::x", Write);
        apply_namespace(
            &mut state,
            &NamespaceTransition::Delete {
                namespace: NamespaceTransitionTarget::Named(TransitionSubject::Literal(
                    "::N".to_owned(),
                )),
            },
            20,
        );
        state.known_namespaces.insert("::N".to_owned());
        assert!(!resolve_literal_access("::N::x", &state, false, &registry, Write).observed);
    }

    #[test]
    fn repeated_root_store_allocates_current_lifetime_without_reviving_dangling_element_alias() {
        let registry = CommandRegistry::build_default();
        let mut initial = ResolveContext::for_function("::f");
        link(&mut initial, "a(k)", "element");
        let mut repeated = initial.clone();
        destroy_root(&mut repeated, "a", 10, &registry);
        initial.join(&repeated);
        let before = resolve_literal_place("a", &initial, false, &registry);
        assert_eq!(
            before.cell.as_ref().map(|cell| cell.generation),
            Some(CellGeneration::Unknown)
        );
        establish_written_lifetime(&mut initial, &before, 20);
        assert_eq!(
            resolve_literal_place("a", &initial, false, &registry)
                .cell
                .as_ref()
                .map(|cell| cell.generation),
            Some(CellGeneration::After(20))
        );
        assert!(initial.unknown_bindings.contains("element"));
    }
    #[test]
    fn jim_aliases_follow_names_after_array_and_namespace_destruction() {
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let mut state = ResolveContext::for_function("::f");
        state.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        ));
        link(&mut state, "a(k)", "element");
        destroy_root(&mut state, "a", 10, registry);
        assert!(!state.unknown_bindings.contains("element"));
        assert_eq!(
            resolve_literal_place("element", &state, false, registry).cell,
            resolve_literal_place("a(k)", &state, false, registry).cell
        );
        bind_alias(
            &mut state,
            &VariableCellAliasTransition {
                destination: tcl_registry::VariableAliasDestination::CurrentNamespaceOrLocal,
                local: TransitionSubject::Literal("global_alias".to_owned()),
                target: VariableAliasTarget::Global {
                    variable: TransitionSubject::Literal("::N::x".to_owned()),
                },
                writes_value: false,
            },
            registry,
        );
        apply_namespace(
            &mut state,
            &NamespaceTransition::Delete {
                namespace: NamespaceTransitionTarget::Named(TransitionSubject::Literal(
                    "::N".to_owned(),
                )),
            },
            20,
        );
        assert!(!state.unknown_bindings.contains("global_alias"));
        assert_eq!(
            resolve_literal_place("global_alias", &state, false, registry).cell,
            resolve_literal_place("::N::x", &state, false, registry).cell
        );
    }
    #[test]
    fn actual_parent_frame_frames_select_bound_cells_and_literal_values() {
        let registry = CommandRegistry::build_default();
        let mut global = ResolveContext::for_function("::top");
        global.define_literal("seed", "GLOBAL", &registry);
        let mut parent_frame =
            global.enter_called_frame(&crate::var_resolve::VariableExecutionFrame::Procedure {
                namespace: "::N".to_owned(),
                identity: "parent_frame@1".to_owned(),
            });
        parent_frame.define_literal("seed", "CALLER", &registry);
        let mut child_frame = parent_frame.enter_called_frame(
            &crate::var_resolve::VariableExecutionFrame::Procedure {
                namespace: "::N".to_owned(),
                identity: "child_frame@1".to_owned(),
            },
        );
        bind_alias(
            &mut child_frame,
            &VariableCellAliasTransition {
                destination: tcl_registry::VariableAliasDestination::CurrentNamespaceOrLocal,
                local: TransitionSubject::Literal("alias".to_owned()),
                target: VariableAliasTarget::CallerSelectedFrame {
                    frame: CallerFrameSelection::DefaultCaller,
                    variable: TransitionSubject::Literal("seed".to_owned()),
                },
                writes_value: false,
            },
            &registry,
        );
        assert_eq!(
            child_frame.literal_value("alias", &registry),
            Some("CALLER")
        );
        assert_eq!(
            child_frame.substitution_literal_value("${alias}", &registry),
            Some("CALLER")
        );
        assert_eq!(
            child_frame
                .selected_frame_context(FrameLevel::Absolute(0))
                .unwrap()
                .literal_value("seed", &registry),
            Some("GLOBAL")
        );
        assert!(
            child_frame
                .selected_frame_context(FrameLevel::Relative(3))
                .is_none()
        );
        let lexical = global.in_frame(&crate::var_resolve::VariableExecutionFrame::Procedure {
            namespace: "::N".to_owned(),
            identity: "definition-only".to_owned(),
        });
        assert!(
            lexical
                .selected_frame_context(FrameLevel::Relative(1))
                .is_none()
        );
    }

    #[test]
    fn namespace_aliases_persist_and_deleted_targets_do_not_rebind_in_c_tcl() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut parent = ResolveContext::for_function("::p");
        parent.namespace_cells.closed = true;
        parent
            .known_namespaces
            .extend(["::N".to_owned(), "::Target".to_owned()]);
        parent.define_literal("::Target::x", "OLD", registry);
        let mut child = parent.enter_called_frame(
            &crate::var_resolve::VariableExecutionFrame::NamespaceActivation {
                namespace: "::N".to_owned(),
                identity: "namespace-eval@1".to_owned(),
            },
        );
        bind_alias(
            &mut child,
            &VariableCellAliasTransition {
                destination: tcl_registry::VariableAliasDestination::CurrentNamespaceOrLocal,
                local: TransitionSubject::Literal("link".to_owned()),
                target: VariableAliasTarget::Global {
                    variable: TransitionSubject::Literal("::Target::x".to_owned()),
                },
                writes_value: false,
            },
            registry,
        );
        let restored = crate::var_resolve::restore_execution_frame(&parent, &child);
        assert_eq!(restored.literal_value("::N::link", registry), Some("OLD"));
        let mut child = restored.enter_called_frame(
            &crate::var_resolve::VariableExecutionFrame::NamespaceActivation {
                namespace: "::N".to_owned(),
                identity: "namespace-eval@2".to_owned(),
            },
        );
        apply_namespace(
            &mut child,
            &NamespaceTransition::Delete {
                namespace: NamespaceTransitionTarget::Named(TransitionSubject::Literal(
                    "::Target".to_owned(),
                )),
            },
            20,
        );
        child.known_namespaces.insert("::Target".to_owned());
        child.define_literal("::Target::x", "NEW", registry);
        let restored = crate::var_resolve::restore_execution_frame(&restored, &child);
        assert_eq!(restored.literal_value("::Target::x", registry), Some("NEW"));
        assert!(
            crate::var_resolve::canonical_literal_variable_name("::N::link", &restored, registry)
                .is_none()
        );
    }

    #[test]
    fn actual_namespace_activation_contributes_to_numeric_frame_depth() {
        let global = ResolveContext::for_function("::top");
        let namespace = global.enter_called_frame(
            &crate::var_resolve::VariableExecutionFrame::NamespaceActivation {
                namespace: "::N".to_owned(),
                identity: "namespace-eval@1".to_owned(),
            },
        );
        let procedure =
            namespace.enter_called_frame(&crate::var_resolve::VariableExecutionFrame::Procedure {
                namespace: "::N".to_owned(),
                identity: "call@1".to_owned(),
            });
        assert!(
            procedure
                .selected_frame_context(FrameLevel::Absolute(1))
                .unwrap()
                .namespace_scope()
        );
        assert!(
            procedure
                .selected_frame_context(FrameLevel::Relative(2))
                .unwrap()
                .global_frame()
        );
        assert!(
            procedure
                .selected_frame_context(FrameLevel::Absolute(3))
                .is_none()
        );
    }
    #[test]
    fn scalar_alias_join_preserves_binding_contents_without_faking_a_root_lifetime() {
        let registry = CommandRegistry::build_default();
        let mut initial = ResolveContext::for_function("::p");
        link(&mut initial, "x", "alias");
        let mut repeated = initial.clone();
        destroy_root(&mut repeated, "x", 10, &registry);
        initial.join(&repeated);
        assert!(!initial.unknown_bindings.contains("alias"));
        let alias = resolve_literal_place("alias", &initial, false, &registry);
        assert_eq!(
            alias.cell.as_ref().map(|cell| cell.generation),
            Some(CellGeneration::Unknown)
        );
        assert_eq!(
            crate::var_resolve::canonical_binding_value_name(&alias),
            crate::var_resolve::canonical_literal_variable_name("x", &initial, &registry)
        );
        assert!(crate::var_resolve::canonical_place_name(&alias).is_none());
        establish_written_lifetime(&mut initial, &alias, 20);
        initial.define_literal("alias", "NEW", &registry);
        assert_eq!(initial.literal_value("x", &registry), Some("NEW"));
    }

    #[test]
    fn actual_parent_frame_retains_outward_alias_and_contents_effects() {
        let registry = CommandRegistry::build_default();
        let mut parent_frame = ResolveContext::for_function("::p");
        parent_frame.define_literal("x", "OLD", &registry);
        link(&mut parent_frame, "x", "alias");
        let child_frame = parent_frame.enter_called_frame(
            &crate::var_resolve::VariableExecutionFrame::Procedure {
                namespace: "::".to_owned(),
                identity: "call@1".to_owned(),
            },
        );
        let mut selected = child_frame
            .selected_frame_context(FrameLevel::Relative(1))
            .unwrap();
        link(&mut selected, "z", "alias");
        selected.define_literal("alias", "NEW", &registry);
        let restored_child_frame =
            crate::var_resolve::restore_execution_frame(&child_frame, &selected);
        let restored_parent_frame =
            crate::var_resolve::restore_execution_frame(&parent_frame, &restored_child_frame);
        let selected_parent_frame = restored_child_frame
            .selected_frame_context(FrameLevel::Relative(1))
            .unwrap();
        assert_eq!(
            selected_parent_frame.literal_value("alias", &registry),
            Some("NEW")
        );
        assert_eq!(
            selected_parent_frame.literal_value("x", &registry),
            Some("OLD")
        );
        // The return adapter must retain binding retargeting published on the
        // actual selected parent_frame, not copy the child_frame's own local alias map.
        assert_eq!(
            restored_parent_frame.literal_value("alias", &registry),
            Some("NEW")
        );
    }
    #[test]
    fn global_declarations_do_not_override_c_namespace_lookup() {
        let registry = CommandRegistry::build_default();
        for version in [
            tcl_dialect::TclVersion::V8_4,
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut state = ResolveContext::for_namespace("::N");
            state.invocation_dialect = Some(tcl_registry::InvocationDialect::for_version(version));
            state.namespace_cells.closed = true;
            state
                .namespace_cells
                .present
                .extend(["::x".to_owned(), "::N::x".to_owned()]);
            bind_alias(
                &mut state,
                &VariableCellAliasTransition {
                    destination: tcl_registry::VariableAliasDestination::ProcedureLocal,
                    local: TransitionSubject::Literal("x".to_owned()),
                    target: VariableAliasTarget::Global {
                        variable: TransitionSubject::Literal("x".to_owned()),
                    },
                    writes_value: false,
                },
                &registry,
            );
            assert!(!state.alias_bindings.contains_key("x"));
            assert_eq!(
                cell_key(&resolve_literal_place("x", &state, false, &registry)),
                crate::var_resolve::VariableCellKey::Authored("::N::x".into())
            );
        }
    }

    #[test]
    fn namespace_alias_destination_does_not_retarget_a_same_named_global() {
        let registry = CommandRegistry::build_default();
        let mut state = ResolveContext::for_namespace("::N");
        state.invocation_dialect = Some(tcl_registry::InvocationDialect::for_version(
            tcl_dialect::TclVersion::V8_6,
        ));
        state.namespace_cells.closed = true;
        state.namespace_cells.present.insert("::link".to_owned());
        bind_alias(
            &mut state,
            &VariableCellAliasTransition {
                destination: tcl_registry::VariableAliasDestination::CurrentNamespaceOrLocal,
                local: TransitionSubject::Literal("link".to_owned()),
                target: VariableAliasTarget::Global {
                    variable: TransitionSubject::Literal("::Target::x".to_owned()),
                },
                writes_value: false,
            },
            &registry,
        );
        assert!(state.namespace_alias_bindings.contains_key("::N::link"));
        assert!(!state.namespace_alias_bindings.contains_key("::link"));
        assert_eq!(
            cell_key(&resolve_literal_place("::link", &state, false, &registry)),
            crate::var_resolve::VariableCellKey::Authored("::link".into())
        );
        assert_eq!(
            cell_key(&resolve_literal_place(
                "::N::link",
                &state,
                false,
                &registry
            )),
            crate::var_resolve::VariableCellKey::Authored("::Target::x".into())
        );
    }
    #[test]
    fn no_value_list_append_preserves_existing_cells_and_initialises_missing_cells() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let registry = tcl_registry::model::ingress::static_context_for(environment).commands();
            let dialect = tcl_registry::InvocationDialect::of_profile(
                tcl_registry::model::ingress::resolve_environment(environment).unit_profile(),
            );
            let mut state = ResolveContext::for_function("::p");
            state.invocation_dialect = Some(dialect);
            state.define_literal("x", "OLD", registry);
            let arguments =
                tcl_registry::InvocationArguments::literals(&["x"]).with_dialect(dialect);
            let facts = registry
                .resolve_invocation("lappend", &["x"], dialect.authoring_query())
                .expect("native no-value form")
                .facts();
            let before = resolve_literal_place("x", &state, false, registry);
            let origin = state.read_contents_origin(&before, registry);
            assert_eq!(
                source_variable_write_places(&facts, arguments, &state, registry).len(),
                0
            );
            transfer_source_namespace_cells(&mut state, &facts, arguments, registry, false, 50);
            assert_eq!(state.literal_value("x", registry), Some("OLD"));
            assert_eq!(state.read_contents_origin(&before, registry), origin);
            let missing =
                tcl_registry::InvocationArguments::literals(&["new"]).with_dialect(dialect);
            let missing_facts = registry
                .resolve_invocation("lappend", &["new"], dialect.authoring_query())
                .expect("native no-value form")
                .facts();
            let target = resolve_literal_place("new", &state, false, registry);
            state.contents_presence.insert(
                crate::var_resolve::cell_key(&target),
                crate::var_resolve::ContentsPresence::Undefined,
            );
            transfer_source_namespace_cells(
                &mut state,
                &missing_facts,
                missing,
                registry,
                false,
                60,
            );
            assert_eq!(
                state.literal_value("new", registry),
                Some(""),
                "{environment}"
            );
        }
    }
}
