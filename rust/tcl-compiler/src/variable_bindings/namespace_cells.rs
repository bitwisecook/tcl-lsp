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

//! Projecting a proved source invocation onto the shared namespace-cell allocation: literal stores, contents writes and state transitions.

use super::OriginalVariableInvocation;
use super::alias_binding::bind_alias_with_compiled_local;
use super::alias_targets::{alias_target, original_alias_target};
use super::cell_lifetimes::{
    destroy_captured_cell, destroy_root, detach_retained_destructions, establish_written_lifetime,
    invalidate_literal_writes,
};
use super::invocation_transfer::{apply_transition, conditional_contents_write};
use super::source_places::{
    contents_write_operand_impl, default_variable_output_order, output_commitment,
    raw_static_unset_operand_error, source_variable_read_places_with_original_operands_impl,
    source_variable_write_places_with_output_order_and_original_operands_impl,
    uncertain_variable_output_addresses_with_original_operands_impl, variable_operand_access_impl,
};
use super::variable_traces::apply_trace_with_original;
use crate::place;
use crate::place::{CellGeneration, Place, PlaceKind};
use crate::var_resolve::{ResolveContext, cell_key, project_access};
use tcl_registry::native_rmw::NativeRmwDialect;
use tcl_registry::{
    CommandRegistry, NamespaceTransition, NamespaceTransitionTarget, StateTransition,
    StateTransitionKnowledge, Traits,
};

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
    callback_accounted: bool,
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
            callback_accounted: false,
            source_offset,
            output_order,
            original_locals: &[],
            operands: None,
            namespace_operations: crate::command_binding::original_namespace_ensure::OriginalNamespaceCellOperations::default(),
        }
    }

    /// The selected handler's independently proved callback boundary.
    pub(crate) fn with_callback_accounted(mut self, callback_accounted: bool) -> Self {
        self.callback_accounted = callback_accounted;
        self
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
        callback_accounted,
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
        && !callback_accounted
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
                state.allocate_namespace_alias_target(&target);
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
    let present = registry
        .special_vars_for_dialect(query)
        .filter(|variable| {
            registry
                .startup_existence(variable.name, query)
                .initially_bound
        })
        // The native entry proof here is for the root table. Named namespace
        // members need the allocated identity of their containing namespace.
        .filter(|variable| {
            tcl_syntax::naming::key_holder_and_tail(variable.name)
                .0
                .is_empty()
        })
        // A global is addressed by its authored spelling or by its cell in
        // the root namespace table; the interpreter creates one variable.
        .flat_map(|variable| {
            [
                crate::var_resolve::VariableCellKey::Authored(tcl_syntax::naming::qualify(
                    "::",
                    variable.name,
                )),
                crate::var_resolve::VariableCellKey::Namespace {
                    identity: crate::command_binding::SourceNamespaceKey::authored("::"),
                    simple: tcl_core_types::NameBytes::from(variable.name.as_bytes()),
                },
            ]
        })
        .collect();
    crate::var_resolve::NamespaceCellPresence {
        present,
        possible: crate::var_resolve::VariableCellSet::default(),
        closed: true,
        closed_namespaces: crate::var_resolve::VariableNamespaceSet::default(),
    }
}
