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

//! Registry semantic calls: the evidence and decline types, the collector, and the binding and trace queries on one semantic operation.

use super::direct_calls::DirectProcDecision;
use super::{AotMetadataSelection, CallCandidate, DirectCallSiteId, call_sites, function_units};
use crate::command_binding::BindingKind;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::registry_invocation::{
    InvocationMetadataContext, RegistryInvocationResolution,
    resolve_command_tokens_with_metadata_context,
};
use crate::semantic_optimisation::{SemanticOptimisationConfig, SemanticOptimisationPassId};
use std::collections::BTreeMap;
use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{DispatchDependencies, SemanticOperationId, Traits};

/// Whether every registry spelling of one semantic operation retains its
/// declared binding in this module.
///
/// This target-neutral query is shared by common proof construction and the
/// backends. Consumers never name Tcl commands themselves.
#[must_use]
pub fn semantic_operation_binding_is_trusted(
    registry: &tcl_registry::CommandRegistry,
    mutations: &crate::command_binding::ModuleCommandMutations,
    operation: SemanticOperationId,
) -> bool {
    let mut commands = registry
        .command_names_for_semantic_operation(operation)
        .peekable();
    commands.peek().is_some() && commands.all(|command| mutations.trusts(command))
}

/// Mapping from an outer registry invocation argument to its evaluated source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticCallArgument {
    /// One ordinary already-evaluated outer argument.
    Value {
        /// Zero-based index after the command head.
        outer_argument: u32,
    },
    /// This outer argument is exactly the result of a selected nested direct
    /// call; substitutions are evaluated once and are never replayed.
    NestedDirect {
        /// Zero-based index after the command head.
        outer_argument: u32,
        /// Exact selected nested call site.
        call: DirectCallSiteId,
    },
}

/// Stable registry form identity selected from structured invocation words.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticFormIdentity(String);

impl SemanticFormIdentity {
    /// Stable registry form spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Registry-resolved, binding-proved semantic call evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticCallEvidence {
    /// Registry-owned operation identity selected from retained source words.
    pub operation: SemanticOperationId,
    /// Registry form identity, when a determinate form matched.
    pub form: Option<SemanticFormIdentity>,
    /// Explicit standalone context, when the caller supplied one.
    pub context: Option<SemanticContext>,
    /// Complete availability used for registry resolution.
    pub metadata_context: tcl_registry::model::ResolvedContext,
    /// Mutable interpreter domains a runtime guard must cover.
    pub dispatch_dependencies: DispatchDependencies,
    /// Registry-declared behaviour traits used by boundary coverage proofs.
    pub traits: Traits,
    /// Exact outer-to-nested argument relationship.
    pub arguments: Vec<SemanticCallArgument>,
}

/// Why an outer call could not receive semantic-operation evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticCallDecline {
    /// The pass is independently disabled by default.
    PassDisabled,
    /// No resolved environment was carried by the unit's semantic bundle.
    ContextUnavailable,
    /// Compatibility lowering did not retain structured invocation words.
    TokensUnavailable,
    /// Structured registry resolution could not select an exact operation/form.
    RegistryUnresolved,
    /// The operation is the generic invocation fallback, not a specialisation.
    GenericInvocation,
    /// Flow-sensitive binding no longer names the original registry command.
    BindingNotOriginal {
        /// Flow-sensitive binding class observed at the call site.
        kind: BindingKind,
    },
    /// A dynamic command-table mutation invalidates all static bindings.
    DynamicCommandMutation,
    /// Whole-module mutation analysis found a possible rebinding.
    ReboundOrAliased,
    /// A dynamic execution trace can observe any call or step.
    DynamicExecutionTrace,
    /// An execution trace, including step traces, observes this command.
    ExecutionTrace,
    /// A nested substitution exists but its exact direct call was not selected.
    NestedDirectUnavailable {
        /// Zero-based outer argument containing the substitution.
        outer_argument: u32,
    },
}

/// Selection or typed decline for one outer semantic call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticCallDecision {
    /// Common analysis proved registry semantics and live binding identity.
    Selected(Box<SemanticCallEvidence>),
    /// The call remains a general Tcl boundary.
    Declined(SemanticCallDecline),
}

pub(super) fn collect_semantic_calls(
    unit: &CompilationUnit,
    registry: &tcl_registry::CommandRegistry,
    selection: AotMetadataSelection,
    config: SemanticOptimisationConfig,
    mutations: &crate::command_binding::ModuleCommandMutations,
    direct_calls: &BTreeMap<DirectCallSiteId, DirectProcDecision>,
) -> BTreeMap<DirectCallSiteId, SemanticCallDecision> {
    let mut decisions = BTreeMap::new();
    for (caller, function) in function_units(unit) {
        for site in call_sites(unit, caller, function, function.source_lexer_config()) {
            if site.id.nested_argument.is_some() {
                continue;
            }
            let decision = semantic_call_decision(&SemanticCallInputs {
                unit,
                registry,
                function,
                context: selection.standalone_context(),
                metadata: selection.for_function(function, registry, &unit.ir_module),
                site: &site,
                mutations,
                direct_calls,
                enabled: config
                    .is_enabled(SemanticOptimisationPassId::SemanticOperationSpecialisation),
            });
            decisions.insert(site.id, decision);
        }
    }
    decisions
}

struct SemanticCallInputs<'a> {
    pub(super) unit: &'a CompilationUnit,
    pub(super) registry: &'a tcl_registry::CommandRegistry,
    pub(super) function: &'a FunctionUnit,
    pub(super) context: Option<SemanticContext>,
    pub(super) metadata: Option<InvocationMetadataContext<'a>>,
    pub(super) site: &'a CallCandidate<'a>,
    pub(super) mutations: &'a crate::command_binding::ModuleCommandMutations,
    pub(super) direct_calls: &'a BTreeMap<DirectCallSiteId, DirectProcDecision>,
    pub(super) enabled: bool,
}

fn semantic_call_decision(input: &SemanticCallInputs<'_>) -> SemanticCallDecision {
    let decline = |reason| SemanticCallDecision::Declined(reason);
    if !input.enabled {
        return decline(SemanticCallDecline::PassDisabled);
    }
    if input.metadata.is_none() {
        return decline(SemanticCallDecline::ContextUnavailable);
    }
    if input.mutations.has_dynamic_mutation() {
        return decline(SemanticCallDecline::DynamicCommandMutation);
    }
    let Some(tokens) = input.site.tokens.as_deref() else {
        return decline(SemanticCallDecline::TokensUnavailable);
    };
    let Ok(RegistryInvocationResolution::Resolved(facts)) =
        resolve_command_tokens_with_metadata_context(input.registry, input.metadata, tokens)
    else {
        return decline(SemanticCallDecline::RegistryUnresolved);
    };
    if facts.operation == SemanticOperationId::Invoke {
        return decline(SemanticCallDecline::GenericInvocation);
    }
    let binding = tokens.source_binding.as_ref();
    if binding
        .and_then(|binding| binding.direct_registry_target(&input.site.command))
        .is_none()
    {
        let kind = binding
            .and_then(crate::command_binding::SourceInvocationBinding::called_slot_kind)
            .unwrap_or(BindingKind::Unknown);
        return decline(SemanticCallDecline::BindingNotOriginal { kind });
    }
    if !input.mutations.trusts(&facts.canonical_command) {
        return decline(SemanticCallDecline::ReboundOrAliased);
    }
    if input.unit.ir_module.has_dynamic_trace {
        return decline(SemanticCallDecline::DynamicExecutionTrace);
    }
    if crate::registry_invocation::original_invocation_has_execution_trace(
        input.registry,
        input.metadata.expect("selected metadata prerequisite"),
        tokens,
        facts.descriptor_key(),
        &input.unit.ir_module.traced_commands,
    ) {
        return decline(SemanticCallDecline::ExecutionTrace);
    }
    let mut arguments = Vec::with_capacity(input.site.args.len());
    for (index, argument) in input.site.args.iter().enumerate() {
        let outer_argument = u32::try_from(index).unwrap_or(u32::MAX);
        if crate::value_shapes::parse_command_substitution_with_config(
            argument,
            input.function.source_lexer_config(),
        )
        .is_some()
        {
            let nested = DirectCallSiteId {
                module_id: input.site.id.module_id,
                body_id: input.site.id.body_id,
                function: input.site.id.function.clone(),
                block: input.site.block,
                statement_index: input.site.statement_index,
                nested_argument: Some(outer_argument),
            };
            if !matches!(
                input.direct_calls.get(&nested),
                Some(DirectProcDecision::Selected(_))
            ) {
                return decline(SemanticCallDecline::NestedDirectUnavailable { outer_argument });
            }
            arguments.push(SemanticCallArgument::NestedDirect {
                outer_argument,
                call: nested,
            });
        } else {
            arguments.push(SemanticCallArgument::Value { outer_argument });
        }
    }
    SemanticCallDecision::Selected(Box::new(SemanticCallEvidence {
        operation: facts.operation,
        form: facts.form.clone().map(SemanticFormIdentity),
        context: input.context,
        metadata_context: input
            .metadata
            .expect("selected metadata prerequisite")
            .context()
            .clone(),
        dispatch_dependencies: facts.dispatch_dependencies,
        traits: facts.traits,
        arguments,
    }))
}

fn command_has_execution_trace(
    registry: &tcl_registry::CommandRegistry,
    module: &crate::ir::Module,
    command: tcl_registry::CanonicalCommandDescriptorKey<'_>,
) -> bool {
    // An implicit semantic operation has no actual called-slot receipt.
    // Native inventories require token history; ambiguous authored descriptor
    // punctuation likewise supplies no constructed-key encoding contract.
    crate::registry_invocation::trace_set_requires_original_owner(
        registry,
        module,
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, module),
    ) || (!module.traced_commands.is_empty() && tcl_syntax::naming::is_absolute(command.label()))
        || module.traced_commands.contains(command.label())
}

pub(super) fn semantic_operation_has_execution_trace(
    registry: &tcl_registry::CommandRegistry,
    module: &crate::ir::Module,
    operation: SemanticOperationId,
) -> bool {
    registry
        .command_descriptor_keys_for_semantic_operation(operation)
        .any(|command| command_has_execution_trace(registry, module, command))
}
