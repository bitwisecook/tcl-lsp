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

//! The normal transfer of an invocation: variable outputs, container writes and operand places.

use super::handler_invocation::resolved_handler_invocation_with_metadata_context;
use super::metadata_context::InvocationMetadataContext;
use super::word_resolution::EffectiveInvocationWord;
use super::{ResolvedStatementInvocation, resolved_tokens_invocation_with_metadata_context};
use crate::ir::CommandTokens;
use tcl_registry::CommandRegistry;
use tcl_registry::model::semantic::SemanticContext;
use tcl_source::words::WordExpr;

/// A reached handler's normal variable transfer, independent of compiler-hook
/// selection. This carrier cannot license an opcode, body evaluation or command
/// completion: its registry facts remain private to the variable transfer owner.
#[derive(Debug)]
pub struct NormalTransferInvocation {
    pub(super) invocation: ResolvedStatementInvocation,
    compilation: tcl_registry::native_compilation::NativeCompilationSelection,
    alias_frame: tcl_registry::VariableAliasFrame,
    captured_outputs: bool,
    original_operands: Option<std::sync::Arc<crate::variable_bindings::OriginalVariableInvocation>>,
}

/// Element evolution of one proved normal, unobserved container store.
pub struct NormalContainerElementWrite<'a> {
    /// Authored element evolution on the normal store continuation.
    pub effect: tcl_registry::VarElementsEffect,
    /// Actual literal variable operand bound to the proved unobserved cell.
    pub target: String,
    /// Effective argv offset introduced by a selected subcommand.
    pub argument_offset: usize,
    /// Retained operand presentation for source-aware value typing. This is
    /// not evidence of evaluated key bytes.
    pub arguments: Vec<String>,
    frozen_dictionary_keys: Option<Vec<String>>,
    /// Effective source words for representation-aware value typing.
    pub words: &'a [WordExpr],
}

impl NormalContainerElementWrite<'_> {
    /// Actual dictionary update key path frozen after argument evaluation.
    /// Unknown evaluated keys cannot borrow their source presentation.
    #[must_use]
    pub fn dictionary_keys(&self) -> Option<&[String]> {
        (self.effect == tcl_registry::VarElementsEffect::SetsDictValue)
            .then_some(self.frozen_dictionary_keys.as_deref())
            .flatten()
    }
}

impl NormalTransferInvocation {
    /// Alias declarations active in the proved frame on the normal continuation.
    /// Native no-op declarations and unproved activation policies are excluded.
    /// This exposes no compiler selection, body execution, or completion facts.
    pub fn variable_alias_transitions(
        &self,
    ) -> impl Iterator<Item = &tcl_registry::state_transition::VariableCellAliasTransition> {
        self.invocation
            .facts
            .state_transitions
            .declared()
            .into_iter()
            .flat_map(tcl_registry::StateTransitions::facts)
            .filter_map(|fact| match &fact.transition {
                tcl_registry::state_transition::StateTransition::VariableCellAlias(alias)
                    if !self.captured_outputs
                        && self.invocation.dialect.and_then(|dialect| {
                            alias
                                .destination
                                .is_active_in_frame(self.alias_frame, dialect)
                        }) == Some(true) =>
                {
                    Some(alias)
                }
                _ => None,
            })
    }

    /// Decode an alias frame selector under this invocation's retained policies.
    /// Missing selector values or execution dialects retain unknown selection.
    #[must_use]
    pub fn variable_alias_frame_level(
        &self,
        frame: &tcl_registry::CallerFrameSelection,
    ) -> Option<tcl_registry::FrameLevel> {
        match frame {
            tcl_registry::CallerFrameSelection::DefaultCaller => {
                Some(tcl_registry::FrameLevel::DEFAULT)
            }
            tcl_registry::CallerFrameSelection::Explicit(level) => {
                tcl_registry::FrameLevel::parse_for_dialect(
                    level.literal()?,
                    self.invocation.dialect?,
                )
            }
        }
    }

    /// Variable trace-table transitions on the reached normal registration path.
    /// Captured output boundaries cannot register or remove an observer.
    pub fn variable_trace_transitions(
        &self,
    ) -> impl Iterator<Item = &tcl_registry::TraceTransition> {
        self.invocation
            .facts
            .state_transitions
            .declared()
            .into_iter()
            .flat_map(tcl_registry::StateTransitions::facts)
            .filter_map(|fact| match &fact.transition {
                tcl_registry::StateTransition::Trace(trace)
                    if !self.captured_outputs
                        && matches!(
                            trace,
                            tcl_registry::TraceTransition::Add {
                                target: tcl_registry::TraceTarget::Variable(_),
                                ..
                            } | tcl_registry::TraceTransition::Remove {
                                target: tcl_registry::TraceTarget::Variable(_),
                                ..
                            }
                        ) =>
                {
                    Some(trace)
                }
                _ => None,
            })
    }

    /// Select container value effects only after proving the actual store cell.
    /// This projection carries no compiler, callback-body or completion licence.
    #[must_use]
    pub fn container_element_write(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Option<NormalContainerElementWrite<'_>> {
        let effect = self.invocation.facts.var_elements_effect?;
        let places = self.mutation_places(state, registry);
        let [place] = places.as_slice() else {
            return None;
        };
        if place.observed || place.dynamic || place.kind == crate::place::PlaceKind::Unknown {
            return None;
        }
        let argument_offset = self.invocation.facts.argument_offset;
        Some(NormalContainerElementWrite {
            effect,
            target: self.invocation.argument_literal(argument_offset)?,
            argument_offset,
            arguments: self
                .invocation
                .arguments
                .iter()
                .cloned()
                .collect::<Option<Vec<_>>>()?,
            frozen_dictionary_keys: (argument_offset + 1
                ..self.invocation.arguments.len().checked_sub(1)?)
                .map(|index| self.invocation.argument_literal(index))
                .collect(),
            words: self.invocation.effective.words.get(1..)?,
        })
    }

    /// Select variable addresses at the phase authored by the reached handler.
    #[must_use]
    pub fn variable_binding_phase(
        &self,
    ) -> tcl_registry::native_compilation::VariableOperandBindingPhase {
        self.invocation.facts.successful_handler.map_or(
            tcl_registry::native_compilation::VariableOperandBindingPhase::AfterArguments,
            tcl_registry::native_compilation::SuccessfulHandlerSpec::variable_binding_phase,
        )
    }

    /// Authored output-address lookup protocol, without exposing compiler facts.
    #[must_use]
    pub fn variable_output_lookup(
        &self,
    ) -> Option<tcl_registry::native_compilation::VariableOutputLookup> {
        self.invocation
            .facts
            .successful_handler
            .map(tcl_registry::native_compilation::SuccessfulHandlerSpec::variable_output_lookup)
    }

    /// Variable output indices in their observable native write order.
    /// Missing ordering evidence remains unknown rather than choosing an order.
    #[must_use]
    pub fn variable_output_arguments(&self) -> Option<Vec<usize>> {
        use tcl_registry::native_compilation::VariableOutputLookup;
        let writes = || {
            self.variable_roles()
                .into_iter()
                .filter_map(|(index, role)| {
                    (role == tcl_registry::ArgRole::VarWrite).then_some(index)
                })
                .collect::<Vec<_>>()
        };
        match self.variable_output_lookup() {
            Some(VariableOutputLookup::CatchSelected) => self.with_arguments(|arguments| {
                let dialect = self.invocation.dialect?;
                let tcl_registry::catch_invocation::CatchInvocationSelection::Valid(capture) =
                    tcl_registry::catch_invocation::select_catch_invocation(arguments, dialect)
                else {
                    return None;
                };
                Some(
                    capture
                        .output_order(dialect, self.compilation)
                        .indices(capture)?
                        .into_iter()
                        .flatten()
                        .collect(),
                )
            }),
            Some(VariableOutputLookup::Sequential | VariableOutputLookup::SingleTarget) => {
                Some(writes())
            }
            Some(VariableOutputLookup::BodyProtocol) => None,
            None => {
                let writes = writes();
                (writes.len() <= 1).then_some(writes)
            }
        }
    }

    /// Locate a setter's retained value operand on the normal handler path.
    /// An operand dependency cannot select a compiler operation or opcode.
    #[must_use]
    pub fn stored_value_argument(&self) -> Option<usize> {
        (self.invocation.facts.operation
            == tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Set,
            )
            && self.argument_count() == 2)
            .then_some(1)
    }

    /// Selected normal numeric store protocol, without a concrete value or
    /// permission to omit its operand conversion or observer schedule.
    #[must_use]
    pub fn numeric_store_production(
        &self,
    ) -> Option<tcl_registry::native_result::NativeNumericStoreProduction> {
        let contract = self.invocation.facts.native_result?;
        self.with_arguments(|arguments| {
            contract
                .normal_numeric_store_production(arguments, self.invocation.facts.argument_offset)
        })
    }

    /// Variable-access traits; these do not select script bodies or compile hooks.
    #[must_use]
    pub fn variable_traits(&self) -> tcl_registry::Traits {
        self.invocation.facts.traits.intersection(
            tcl_registry::Traits::READS_BEFORE_WRITE
                | tcl_registry::Traits::DESTROYS_VARIABLE
                | tcl_registry::Traits::CONDITIONAL_VARIABLE_WRITE
                | tcl_registry::Traits::WHOLE_ARRAY_ARG,
        )
    }

    /// Locate an original value word without reconstructing frozen argv.
    #[must_use]
    pub fn written_argument(&self, argument: usize) -> Option<usize> {
        self.invocation.effective.written_argument(argument)
    }

    /// Number of effective arguments on this reached handler continuation.
    #[must_use]
    pub fn argument_count(&self) -> usize {
        self.invocation.arguments.len()
    }

    /// Named contents reads on this handler phase, independently of alias registration.
    #[must_use]
    pub fn read_places(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Vec<crate::place::Place> {
        if self.captured_outputs
            || self.variable_binding_phase()
                == tcl_registry::native_compilation::VariableOperandBindingPhase::BodyProtocol
        {
            return Vec::new();
        }
        self.with_arguments(|arguments| {
            if state.execution_name_policy.is_some() {
                return self.original_operands.as_deref().map_or_else(
                    || vec![crate::place::unknown_top()],
                    |operands| crate::variable_bindings::source_variable_read_places_with_original_operands(
                        &self.invocation.facts, arguments, state, registry, operands,
                    ),
                );
            }
            crate::variable_bindings::source_variable_read_places(
                &self.invocation.facts, arguments, state, registry,
            )
        })
    }

    /// A written value receiver whose contents are proved absent before the
    /// required read. Existence probes have a different result contract.
    pub(crate) fn missing_value_read_argument(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Option<(usize, tcl_core_types::NameBytes)> {
        use tcl_registry::native_result::{NativeResultSelection, VariableResultPhase};
        let contract = self.invocation.facts.native_result?;
        let NativeResultSelection::VariableValue {
            variable_at,
            phase: VariableResultPhase::AfterRead,
        } = self.with_arguments(|arguments| {
            contract.select(arguments, self.invocation.facts.argument_offset)
        })
        else {
            return None;
        };
        let origin = self.written_argument(variable_at)?;
        let place = self.original_operands.as_ref()?.access(
            variable_at,
            state,
            registry,
            tcl_registry::TraceOperation::Read,
            false,
        );
        state
            .read_is_definitely_missing(&place, registry)
            .then_some(())?;
        Some((origin, place.cell?.name))
    }

    /// The written target of a selected store rejected by the current cell.
    pub(crate) fn constant_store_argument(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
        stock_object_types: impl FnOnce() -> bool,
    ) -> Option<usize> {
        use tcl_registry::native_compilation::{NativeAppendKind, NativeCompilationGrammar};
        use tcl_syntax::native_string::NativeStringProtocol;
        let protocol = self.invocation.dialect?.native_string_protocol()?;
        let NativeStringProtocol::C(version) = protocol else {
            return None;
        };
        let facts = &self.invocation.facts;
        if version < tcl_dialect::TclVersion::V9_0
            || !facts.arg_roles_complete
            || facts.arity_accepts_frozen_arguments() != Some(true)
        {
            return None;
        }
        let count = self.argument_count().checked_sub(facts.argument_offset)?;
        let reads_list = match facts.native_compilation?.grammar {
            NativeCompilationGrammar::VariableLoadStore
                if self.stored_value_argument().is_some() =>
            {
                false
            }
            // C Tcl 9 rejects increments of constants before converting the amount.
            NativeCompilationGrammar::Increment if matches!(count, 1 | 2) => false,
            NativeCompilationGrammar::VariableAppend(kind) if count > 1 => {
                kind == NativeAppendKind::List
            }
            _ => return None,
        };
        let mut writes = facts
            .arg_roles
            .iter()
            .filter(|(_, role)| *role == tcl_registry::ArgRole::VarWrite);
        let variable_at = facts.argument_offset + usize::from(writes.next()?.0);
        if writes.next().is_some() {
            return None;
        }
        let origin = self.written_argument(variable_at)?;
        let operands = self.original_operands.as_ref()?;
        let place = operands.access(
            variable_at,
            state,
            registry,
            tcl_registry::TraceOperation::Write,
            false,
        );
        state.selected_scalar_is_constant(&place).then_some(())?;
        if reads_list {
            stock_object_types().then_some(())?;
            let read = operands.access(
                variable_at,
                state,
                registry,
                tcl_registry::TraceOperation::Read,
                false,
            );
            // Native lappend reads and validates its old list before the write.
            if read.cell != place.cell || !state.read_produces_value(&read, registry) {
                return None;
            }
            let value = state.literal_contents_at(&read, registry)?;
            let bytes = protocol.host_text_bytes(value);
            tcl_syntax::list::split_native_list_bytes(&bytes, protocol).ok()?;
        }
        Some(origin)
    }

    /// Bound contents changes, including destruction; no runtime dispatch licence.
    #[must_use]
    pub fn mutation_places(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Vec<crate::place::Place> {
        if self.captured_outputs {
            return self.definition_places(state, registry);
        }
        self.with_arguments(|arguments| {
            if state.execution_name_policy.is_some() {
                return self.original_operands.as_deref().map_or_else(
                    || vec![crate::place::unknown_top()],
                    |operands| crate::variable_bindings::source_variable_write_places_with_output_order_and_original_operands(
                        &self.invocation.facts, arguments, state, registry,
                        self.variable_output_arguments().as_deref(), operands,
                    ),
                );
            }
            crate::variable_bindings::source_variable_write_places_with_output_order(
                &self.invocation.facts, arguments, state, registry,
                self.variable_output_arguments().as_deref(),
            )
        })
    }

    /// Bound SSA value definitions on the successful continuation.
    #[must_use]
    pub fn definition_places(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Vec<crate::place::Place> {
        if self.variable_binding_phase()
            == tcl_registry::native_compilation::VariableOperandBindingPhase::BodyProtocol
        {
            return Vec::new();
        }
        self.definition_operands(state, registry)
            .into_iter()
            .map(|(_, place)| place)
            .collect()
    }

    /// Original written output arguments paired with their selected normal
    /// definition places. Inserted and expanded operands have no written slot.
    /// Missing address/order evidence remains unknown in the returned place.
    #[must_use]
    pub fn written_definition_places(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Vec<(usize, crate::place::Place)> {
        if self.variable_binding_phase()
            == tcl_registry::native_compilation::VariableOperandBindingPhase::BodyProtocol
        {
            return Vec::new();
        }
        self.definition_operands(state, registry)
            .into_iter()
            .filter_map(|(effective, place)| Some((self.written_argument(effective)?, place)))
            .collect()
    }

    fn definition_operands(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Vec<(usize, crate::place::Place)> {
        if state.execution_name_policy.is_some() {
            let Some(operands) = self
                .original_operands
                .as_deref()
                .filter(|operands| operands.argument_count() == self.argument_count())
            else {
                return Vec::new();
            };
            return self.with_arguments(|arguments| {
                crate::variable_bindings::source_variable_definitions_with_original_operands(
                    &self.invocation.facts,
                    arguments,
                    state,
                    registry,
                    self.variable_output_arguments().as_deref(),
                    operands,
                )
            });
        }
        crate::place_bridge::resolved_invocation_variable_definitions(
            &self.invocation,
            state,
            registry,
            self.variable_output_arguments().as_deref(),
        )
    }

    /// Apply only the variable owner's normal continuation effects.
    pub fn transfer_variables(
        &self,
        state: &mut crate::var_resolve::ResolveContext,
        statement: &crate::ir::Statement,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
    ) {
        if self.captured_outputs {
            let outputs = self.definition_places(state, registry);
            crate::variable_bindings::transfer_captured_variable_outputs(
                state,
                &outputs,
                statement.span().start(),
            );
            return;
        }
        self.with_arguments(|arguments| {
            crate::variable_bindings::transfer_resolved_invocation(
                state,
                statement,
                tokens,
                &self.invocation.facts,
                arguments,
                registry,
                (
                    self.variable_output_arguments().as_deref(),
                    self.original_operands.as_deref(),
                ),
            );
        });
    }

    /// A passthrough store value only when the selected cell has no write
    /// observer and its physical store is bounded. This says nothing about
    /// executing the invocation or eliminating its possible compiler error.
    ///
    /// `state` must describe the actual address-selection phase returned by
    /// `variable_binding_phase`, including all preceding argv/body effects.
    /// A pre-argv or freshly reconstructed context cannot establish this store.
    #[must_use]
    pub fn stored_value_operand(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Option<(&str, &WordExpr)> {
        let word = self.stored_value_word(state, registry)?;
        Some((self.invocation.arguments.get(1)?.as_deref()?, word))
    }

    /// Original stored word after proving an unobserved bounded normal store.
    /// Unknown evaluated bytes retain their structured source and read identity.
    #[must_use]
    pub fn stored_value_word(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Option<&WordExpr> {
        self.stored_value_argument()?;
        let places = self.mutation_places(state, registry);
        let [place] = places.as_slice() else {
            return None;
        };
        if place.observed || place.dynamic || place.kind == crate::place::PlaceKind::Unknown {
            return None;
        }
        self.invocation.effective.words.get(2)
    }

    /// The value frozen before later argv substitutions, after proving the
    /// same observer-free physical store as `stored_value_operand`. Its actual
    /// address-selection context precondition applies here as well.
    #[must_use]
    pub fn stored_value_literal(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Option<String> {
        self.stored_value_operand(state, registry)?;
        self.invocation.argument_literal(1)
    }

    /// Named variable operands, limited to storage access roles.
    #[must_use]
    pub fn variable_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        self.invocation
            .facts
            .arg_roles
            .iter()
            .filter_map(|&(index, role)| {
                matches!(
                    role,
                    tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarWrite
                )
                .then_some((
                    self.invocation.facts.argument_offset + usize::from(index),
                    role,
                ))
            })
            .collect()
    }

    /// Select one named storage operand from this retained normal invocation.
    /// Original execution uses its exact byte/compiler descriptor; the label
    /// returned by `argument_literal` is never an address producer.
    pub(crate) fn variable_operand_place(
        &self,
        argument: usize,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Option<crate::place::Place> {
        let role = self
            .variable_roles()
            .into_iter()
            .find_map(|(index, role)| (index == argument).then_some(role))?;
        self.with_arguments(|arguments| {
            if role == tcl_registry::ArgRole::VarWrite {
                return if state.execution_name_policy.is_some() {
                    self.original_operands.as_deref().map(|operands| {
                        crate::variable_bindings::variable_output_operand_access_with_original_operands(
                            &self.invocation.facts, arguments, argument, state, registry, operands,
                        )
                    })
                } else {
                    Some(crate::variable_bindings::variable_output_operand_access(
                        &self.invocation.facts, arguments, argument, state, registry,
                    ))
                };
            }
            if state.execution_name_policy.is_some() {
                return self.original_operands.as_deref()
                    .filter(|operands| operands.argument_count() == arguments.len())
                    .map(|operands| operands.access(argument, state, registry,
                        tcl_registry::TraceOperation::Read,
                        self.variable_traits().contains(tcl_registry::Traits::WHOLE_ARRAY_ARG)));
            }
            Some(crate::variable_bindings::variable_operand_access(
                arguments, argument, state, registry, tcl_registry::TraceOperation::Read,
                self.variable_traits().contains(tcl_registry::Traits::WHOLE_ARRAY_ARG),
            ))
        })
    }

    /// One variable operand's frozen name shape. A known array root remains
    /// bounded even when its element index is computed; source spelling and
    /// mathematical contents facts cannot replace this actual argv projection.
    #[must_use]
    pub fn variable_operand_word(&self, index: usize) -> Option<EffectiveInvocationWord> {
        self.variable_roles()
            .iter()
            .any(|&(variable_index, _)| variable_index == index)
            .then(|| self.invocation.argument_word(index))
    }

    /// A frozen variable/value operand on this normal transfer boundary.
    #[must_use]
    pub fn argument_literal(&self, index: usize) -> Option<String> {
        self.invocation.argument_literal(index)
    }

    /// Source names receiving contents on the successful continuation.
    #[must_use]
    pub fn definition_names(&self) -> Vec<String> {
        self.variable_roles()
            .into_iter()
            .filter_map(|(index, role)| {
                if role != tcl_registry::ArgRole::VarWrite {
                    return None;
                }
                let name = self.argument_literal(index)?;
                crate::variable_bindings::contents_write_operand(
                    &self.invocation.facts,
                    Some(&name),
                )
                .then_some(name)
            })
            .collect()
    }

    fn with_arguments<T>(
        &self,
        apply: impl FnOnce(tcl_registry::InvocationArguments<'_>) -> T,
    ) -> T {
        let values: Vec<_> = (0..self.argument_count())
            .map(|index| self.invocation.argument_word(index))
            .collect();
        let words: Vec<_> = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let mut arguments = tcl_registry::InvocationArguments::structured(&words);
        if let Some(dialect) = self.invocation.dialect {
            arguments = arguments.with_dialect(dialect);
        }
        apply(arguments)
    }
}

/// Resolve a normal handler transfer. Strict executable resolution remains the
/// first choice. An unknown compiler protocol additionally needs an exact stable
/// handler and an authored normal-effect equivalence for its frozen argv/frame.
#[must_use]
pub fn normal_transfer_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<NormalTransferInvocation> {
    normal_transfer_invocation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Resolve retained normal transfer metadata under the complete supplied availability generation.
/// Existing original identity, argument and handler premises remain required.
#[must_use]
pub fn normal_transfer_invocation_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Option<NormalTransferInvocation> {
    normal_transfer_invocation_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

/// Select successful transfer metadata with the complete supplied source owner.
/// Missing or foreign input cannot reconstruct a standalone catalogue context.
#[must_use]
pub fn normal_transfer_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<NormalTransferInvocation> {
    if tokens.synthetic == Some(crate::ir::SyntheticMarker::CapturedCatchOutputs) {
        let mut original = tokens.clone();
        original.synthetic = None;
        let mut normal =
            normal_transfer_invocation_with_metadata_context(registry, context, &original)?;
        normal.captured_outputs = true;
        return Some(normal);
    }
    if let Some(invocation) =
        resolved_tokens_invocation_with_metadata_context(registry, context, tokens)
    {
        return Some(NormalTransferInvocation {
            invocation,
            compilation: normal_output_compilation(tokens),
            alias_frame: normal_alias_frame(tokens),
            captured_outputs: false,
            original_operands: tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.original_variable_operands_for_tokens(tokens))
                .cloned(),
        });
    }
    let binding = tokens.source_binding.as_ref()?;
    let invocation = resolved_handler_invocation_with_metadata_context(registry, context, tokens)?;
    let normal = NormalTransferInvocation {
        invocation,
        compilation: normal_output_compilation(tokens),
        alias_frame: normal_alias_frame(tokens),
        captured_outputs: false,
        original_operands: binding
            .original_variable_operands_for_tokens(tokens)
            .cloned(),
    };
    normal.with_arguments(|arguments| {
        normal
            .invocation
            .facts
            .successful_handler_effects(arguments, binding.variable_context.alias_frame())
    })?;
    Some(normal)
}

fn normal_alias_frame(tokens: &CommandTokens) -> tcl_registry::VariableAliasFrame {
    tokens
        .source_binding
        .as_ref()
        .map_or(tcl_registry::VariableAliasFrame::Unknown, |binding| {
            binding.variable_context.alias_frame()
        })
}

fn normal_output_compilation(
    tokens: &CommandTokens,
) -> tcl_registry::native_compilation::NativeCompilationSelection {
    tokens.source_binding.as_ref().map_or(
        tcl_registry::native_compilation::NativeCompilationSelection::Unknown,
        crate::command_binding::SourceInvocationBinding::native_compilation_selection,
    )
}
