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

//! Transitions of one resolved invocation: declared effects, bodies and recovered definitions.

use super::super::namespace_context::NamespaceKeyQuery;
use super::super::trusted_packages::apply_package_transition;
use super::super::{
    InvocationBindingContext, ModuleCommandBindings, command_publication, original_command_table,
};
use super::binding_transitions::apply_may_binding_transition;
use super::binding_walk::BindingInvocationMetadata;
use super::evaluated_bodies::{apply_readable_evaluated_body, procedure_publication_key};
use super::namespace_transitions::apply_namespace_transition;
use super::{ResolvedBindingInvocation, recovered_procedure_source};
use crate::ir::{Module, Statement};
use std::sync::Arc;
use tcl_registry::{
    CommandBindingDefinitionKind, CommandBindingTransition, CommandRegistry, StateTransition,
    StateTransitionDomain,
};

fn invocation_transition_inputs(
    stmt: &Statement,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    command_namespace: &(impl NamespaceKeyQuery + ?Sized),
    source_order_mode: bool,
    metadata: BindingInvocationMetadata<'_>,
) -> (bool, Vec<ResolvedBindingInvocation>, bool) {
    let (Statement::Call { command, .. } | Statement::Barrier { command, .. }) = stmt else {
        return (false, Vec::new(), false);
    };
    let source_may_be_unknown = bindings.target_may_be_unknown(command, command_namespace);
    let reaches_user_procedure = source_order_mode
        && bindings
            .targets(command, command_namespace)
            .iter()
            .any(|target| !target.registry_backed);
    (
        source_may_be_unknown,
        match metadata {
            BindingInvocationMetadata::Standalone => {
                bindings.resolve_statement(stmt, registry, command_namespace)
            }
            BindingInvocationMetadata::Supplied(metadata) => bindings
                .resolve_statement_with_metadata_context(
                    stmt,
                    registry,
                    metadata,
                    command_namespace,
                ),
        },
        reaches_user_procedure,
    )
}

fn join_transition_alternative(
    joined: &mut Option<ModuleCommandBindings>,
    alternative: ModuleCommandBindings,
) {
    if let Some(state) = joined {
        if !state.same_state(&alternative) {
            state.join(&alternative);
        }
    } else {
        *joined = Some(alternative);
    }
}

/// Whether every procedure a recovered definition installs, `qname` among
/// them, already has a retained body, with no unretained executable root.
fn procedure_inventory_complete(
    module: &crate::ir::Module,
    qname: &str,
    bindings: &ModuleCommandBindings,
) -> bool {
    !module.oo_evidence.unretained_executable_roots
        && std::iter::once(qname)
            .chain(
                module
                    .procedures
                    .values()
                    .map(|procedure| procedure.qualified_name.as_str()),
            )
            .all(|name| bindings.procedure_bodies.contains(name))
}

/// Whether `stmt` calls a document-declared command, which is trusted to
/// leave the command world intact, as the source walk trusts it; its stated
/// frame effect bounds what it does to variables.
fn names_a_declared_command(
    stmt: &Statement,
    bindings: &ModuleCommandBindings,
    command_namespace: &(impl NamespaceKeyQuery + ?Sized),
) -> bool {
    matches!(stmt, Statement::Call { command, .. } | Statement::Barrier { command, .. }
        if bindings.names_only_a_declared_command(command, command_namespace))
}

pub(in super::super) fn apply_may_invocation_transitions(
    stmt: &Statement,
    registry: &CommandRegistry,
    bindings: &mut ModuleCommandBindings,
    observed: &mut Option<ModuleCommandBindings>,
    command_namespace: &(impl NamespaceKeyQuery + ?Sized),
    execution_namespace: &crate::ir_helpers::ExecutionNamespace,
    context: &mut InvocationBindingContext<'_>,
) -> bool {
    if names_a_declared_command(stmt, bindings, command_namespace) {
        return false;
    }
    let (source_may_be_unknown, invocations, reaches_user_procedure) = invocation_transition_inputs(
        stmt,
        registry,
        bindings,
        command_namespace,
        context.source_order_mode,
        context.metadata,
    );
    let single_exact_invocation = !source_may_be_unknown && invocations.len() == 1;
    let mut joined: Option<ModuleCommandBindings> = source_may_be_unknown.then(|| bindings.clone());
    let mut exact_definition_key = None;
    for invocation in invocations {
        // A single resolved invocation has no alternative to join. Move the
        // state through that transfer directly so its copy-on-write maps stay
        // uniquely owned; cloning here made a linear run of N exact `proc`
        // declarations copy the growing map N times.
        let mut alternative = if single_exact_invocation {
            std::mem::take(bindings)
        } else {
            bindings.clone()
        };
        let non_binding_before =
            single_exact_invocation.then(|| alternative.non_binding_observation_stamp());
        let procedure_body = recover_procedure_definition(
            &invocation,
            execution_namespace,
            alternative.baseline.dialect,
        );
        let precise_procedure = match &procedure_body {
            ProcedureDefinitionReplay::Recovered { qname, module } => {
                let inventory_was_complete =
                    procedure_inventory_complete(module, qname, &alternative);
                if module.oo_evidence.unretained_executable_roots {
                    alternative.mark_opaque_binding_mutation();
                }
                alternative.extend_procedure_bodies(
                    std::iter::once(qname.clone()).chain(
                        module
                            .procedures
                            .values()
                            .map(|procedure| procedure.qualified_name.clone()),
                    ),
                );
                context.retained_roots.extend_module_roots(module, false);
                if single_exact_invocation && inventory_was_complete {
                    exact_definition_key = exact_procedure_definition_key(
                        &invocation.facts,
                        execution_namespace,
                        qname,
                        alternative.baseline.dialect,
                    );
                }
                Some(qname.as_str())
            }
            ProcedureDefinitionReplay::NotProcedure
            | ProcedureDefinitionReplay::KnownError
            | ProcedureDefinitionReplay::Unavailable => None,
        };
        if !matches!(&procedure_body, ProcedureDefinitionReplay::KnownError) {
            apply_declared_binding_transitions(
                &invocation.facts,
                &mut alternative,
                execution_namespace,
                precise_procedure,
                invocation.source_span.start(),
                None,
                None,
            );
        }
        let body_shape_is_selected = invocation_selects_evaluated_body(&invocation.facts);
        if body_shape_is_selected {
            // An authored Proc-lowering spec may both install a procedure and
            // evaluate a body immediately. That body owns an arbitrary nested
            // binding transfer, so the declared one-key transition no longer
            // proves that the invocation changed only the procedure name.
            // Stock Tcl `proc` carries DEFERS_BODY and stays on the hot path.
            exact_definition_key = None;
        }
        apply_possible_invocation_body(
            &invocation,
            registry,
            &mut alternative,
            observed,
            execution_namespace,
            context,
            body_shape_is_selected,
        );
        if non_binding_before
            .is_some_and(|before| before != alternative.non_binding_observation_stamp())
        {
            // The typed procedure definition also changed another observable
            // axis (for example LOADS_EXTERNAL_UNIT made command bindings
            // opaque). The one-key historical delta is no longer a complete
            // observation, so retain the ordinary full-state join.
            exact_definition_key = None;
        }
        join_transition_alternative(&mut joined, alternative);
    }
    if let Some(state) = joined {
        *bindings = state;
    }
    if reaches_user_procedure {
        bindings.mark_source_order_user_procedure_call();
    }
    observe_exact_procedure_definition(observed, bindings, exact_definition_key)
}

fn apply_possible_invocation_body(
    invocation: &ResolvedBindingInvocation,
    registry: &CommandRegistry,
    alternative: &mut ModuleCommandBindings,
    observed: &mut Option<ModuleCommandBindings>,
    execution_namespace: &crate::ir_helpers::ExecutionNamespace,
    context: &mut InvocationBindingContext<'_>,
    body_shape_is_selected: bool,
) {
    let body_selection_is_indeterminate = invocation_may_select_evaluated_body(&invocation.facts);
    let readable_body_was_applied = body_shape_is_selected
        && apply_readable_evaluated_body(
            invocation,
            registry,
            alternative,
            observed,
            execution_namespace,
            context,
        );
    if invocation_body_requires_widening(
        &invocation.facts,
        body_shape_is_selected,
        body_selection_is_indeterminate,
        readable_body_was_applied,
    ) {
        alternative.mark_opaque_binding_mutation();
    }
}

/// Preserve external loading and unreadable body effects beyond declared transfers.
fn invocation_body_requires_widening(
    facts: &tcl_registry::InvocationFacts,
    body_selected: bool,
    selection_indeterminate: bool,
    readable_body_applied: bool,
) -> bool {
    facts
        .traits
        .contains(tcl_registry::Traits::LOADS_EXTERNAL_UNIT)
        && !facts
            .state_transitions
            .declared()
            .is_some_and(|transitions| {
                transitions
                    .facts()
                    .iter()
                    .any(|fact| matches!(fact.transition, StateTransition::Package(_)))
            })
        || (facts
            .traits
            .contains(tcl_registry::Traits::DYNAMIC_EVAL_BODY)
            && ((!body_selected && selection_indeterminate)
                || (body_selected && !readable_body_applied)))
}

/// Publish one exact definition into the historical view without a full-map
/// lattice join. Returns whether the caller's ordinary observation is already
/// complete.
fn observe_exact_procedure_definition(
    observed: &mut Option<ModuleCommandBindings>,
    bindings: &ModuleCommandBindings,
    key: Option<String>,
) -> bool {
    let Some(key) = key else {
        return false;
    };
    if let Some(history) = observed {
        history.join_binding_from(bindings, &key);
        return true;
    }

    // No earlier invocation supplied the pre-transition state. For this first
    // exact definition that state is the registry baseline; seed it explicitly
    // before joining the new procedure target.
    let mut history = bindings.clone();
    let mut historical = ModuleCommandBindings::unmodified_bindings(
        &key,
        history.baseline.semantics.binding_names(),
    );
    historical.extend(bindings.bindings.get(&key).cloned().unwrap_or_else(|| {
        ModuleCommandBindings::unmodified_bindings(&key, history.baseline.semantics.binding_names())
    }));
    history.replace(key, historical);
    *observed = Some(history);
    true
}

/// The exact, registry-declared procedure binding established by one
/// invocation, when that is its sole state transition. This deliberately keys
/// the fast path on typed transition data rather than the spelling `proc`.
fn exact_procedure_definition_key(
    facts: &tcl_registry::InvocationFacts,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    precise_procedure: &str,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> Option<String> {
    let transitions = facts.state_transitions.declared()?;
    let [fact] = transitions.facts() else {
        return None;
    };
    let StateTransition::CommandBinding(CommandBindingTransition::Define { name, kind }) =
        &fact.transition
    else {
        return None;
    };
    if *kind != CommandBindingDefinitionKind::Procedure {
        return None;
    }
    let key = procedure_publication_key(namespace, name.literal()?, dialect)?;
    (key == precise_procedure).then_some(key)
}

pub(super) fn apply_resolved_may_transitions(
    invocations: Vec<tcl_registry::InvocationFacts>,
    source_may_be_unknown: bool,
    dynamic_body_is_opaque: bool,
    bindings: &mut ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
) {
    let mut joined: Option<ModuleCommandBindings> = source_may_be_unknown.then(|| bindings.clone());
    for facts in invocations {
        let mut alternative = bindings.clone();
        if facts
            .traits
            .contains(tcl_registry::Traits::LOADS_EXTERNAL_UNIT)
            && !facts
                .state_transitions
                .declared()
                .is_some_and(|transitions| {
                    transitions
                        .facts()
                        .iter()
                        .any(|fact| matches!(fact.transition, StateTransition::Package(_)))
                })
            || (dynamic_body_is_opaque
                && facts
                    .traits
                    .contains(tcl_registry::Traits::DYNAMIC_EVAL_BODY)
                && (invocation_selects_evaluated_body(&facts)
                    || invocation_may_select_evaluated_body(&facts)))
        {
            alternative.mark_opaque_binding_mutation();
        }
        apply_declared_binding_transitions(
            &facts,
            &mut alternative,
            namespace,
            None,
            0,
            None,
            None,
        );
        if let Some(state) = &mut joined {
            if !state.same_state(&alternative) {
                state.join(&alternative);
            }
        } else {
            joined = Some(alternative);
        }
    }
    if let Some(state) = joined {
        *bindings = state;
    }
}

/// Whether the resolved invocation selects an argument that Tcl evaluates as
/// script. Some ensemble roots carry `DYNAMIC_EVAL_BODY` for only selected
/// subcommands, so the root trait alone cannot make a precise leaf opaque.
fn invocation_selects_evaluated_body(facts: &tcl_registry::InvocationFacts) -> bool {
    use tcl_registry::SemanticOperationId;
    use tcl_registry::frame_effect::FrameArgLayout;
    use tcl_registry::hooks::LoweringHookId;

    if facts.traits.contains(tcl_registry::Traits::DEFERS_BODY) {
        return false;
    }

    matches!(
        facts.operation,
        SemanticOperationId::StructuredLowering(
            LoweringHookId::Apply | LoweringHookId::NamespaceEval
        )
    ) || facts
        .arg_roles
        .iter()
        .any(|(_, role)| *role == tcl_registry::ArgRole::Body)
        || facts.frame_effect.is_some_and(|effect| {
            matches!(
                effect.layout,
                FrameArgLayout::ScriptInCurrentFrame | FrameArgLayout::ScriptInSelectedFrame
            )
        })
}

/// Whether a resolved registry head still has an invocation-time choice of a
/// body-bearing leaf. The root's broad dynamic-evaluation trait is meaningful
/// for an indeterminate ensemble subcommand, but must not taint an exact
/// non-body or deferred leaf.
fn invocation_may_select_evaluated_body(facts: &tcl_registry::InvocationFacts) -> bool {
    matches!(
        &facts.subcommand,
        tcl_registry::OwnedSubcommandResolution::Indeterminate { .. }
    ) || (!facts.arg_roles_complete && !facts.traits.contains(tcl_registry::Traits::DEFERS_BODY))
}

pub(in super::super) fn apply_declared_binding_transitions(
    facts: &tcl_registry::InvocationFacts,
    bindings: &mut ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    precise_procedure: Option<&str>,
    declaration: u32,
    publication: Option<&command_publication::PreservingCommandPublication>,
    original_operands: Option<&original_command_table::OriginalCommandOperands>,
) {
    let Some(transitions) = facts.state_transitions.declared() else {
        Arc::make_mut(&mut bindings.command_observers)
            .fresh_procedures
            .clear();
        return;
    };
    if transitions.command_bindings().next().is_some()
        || transitions.facts().iter().any(|fact| {
            matches!(
                &fact.transition,
                StateTransition::Trace(_)
                    | StateTransition::Namespace(_)
                    | StateTransition::Interpreter(_)
                    | StateTransition::Package(_)
                    | StateTransition::ObjectDispatch(_)
                    | StateTransition::Widen(_)
            )
        })
    {
        Arc::make_mut(&mut bindings.command_observers)
            .fresh_procedures
            .clear();
    }
    let interpreted_execution =
        facts.body_execution.is_some() || invocation_selects_evaluated_body(facts);
    if !interpreted_execution && transitions.facts().iter().any(|fact| matches!(&fact.transition, StateTransition::Widen(widening) if widening.domains.contains(&StateTransitionDomain::CommandResolution))) {
        bindings.mark_opaque_resolution();
    }
    for fact in transitions.facts() {
        if let StateTransition::Namespace(transition) = &fact.transition {
            bindings.namespace_resolution.record(transition, namespace);
            apply_namespace_transition(
                bindings,
                transition,
                namespace,
                declaration,
                original_operands,
            );
        } else if let StateTransition::Interpreter(transition) = &fact.transition {
            bindings.retain_created_interpreter_command(transition, declaration);
        } else if let StateTransition::Package(transition) = &fact.transition {
            apply_package_transition(bindings, transition);
        } else if let StateTransition::ObjectDispatch(
            tcl_registry::ObjectDispatchTransition::Configure { target, .. },
        ) = &fact.transition
        {
            Arc::make_mut(&mut bindings.object_instances).invalidate_class_delegates(false);
            Arc::make_mut(&mut bindings.object_instances).invalidate_dispatch(false);
            if let (Some(name), crate::ir_helpers::ExecutionNamespace::Exact(namespace)) =
                (target.literal(), namespace)
            {
                let targets = bindings.targets(name, namespace);
                for target in targets {
                    if let Some(identity) = target.token {
                        Arc::make_mut(&mut bindings.class_definitions).remove(&identity);
                        Arc::make_mut(&mut bindings.bounded_constructions).remove(&identity);
                    }
                    Arc::make_mut(&mut bindings.tainted_object_dispatch).insert(target.command);
                }
            } else {
                Arc::make_mut(&mut bindings.tainted_object_dispatch).insert("*".to_owned());
            }
        } else if let StateTransition::Trace(transition) = &fact.transition {
            bindings.retain_command_observer(transition, namespace);
        }
    }
    if !interpreted_execution
        && transitions.facts().iter().any(|fact| {
            matches!(
                &fact.transition,
                StateTransition::Widen(widening)
                    if widening.domains.contains(&StateTransitionDomain::CommandBindings)
            )
        })
    {
        bindings.mark_dynamic_proc_binding();
    }
    for transition in transitions.command_bindings() {
        apply_may_binding_transition(
            bindings,
            transition,
            namespace,
            precise_procedure,
            declaration,
            publication,
            original_operands,
        );
    }
}

pub(in super::super) enum ProcedureDefinitionReplay {
    NotProcedure,
    /// Tcl rejects the invocation before installing a command binding.
    KnownError,
    /// A procedure may be installed, but its runtime body is not source-safe.
    Unavailable,
    Recovered {
        qname: String,
        module: Box<Module>,
    },
}

/// Recover the body installed by the registry's typed procedure-definition
/// operation, including arguments prepended by an alias chain. This is the
/// only path that authorises a precise non-registry command target: a bare
/// `Define(Procedure)` fact is intentionally insufficient because consumers
/// could otherwise dispatch to a procedure whose effects were never walked.
pub(in super::super) fn recover_procedure_definition(
    invocation: &ResolvedBindingInvocation,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> ProcedureDefinitionReplay {
    use tcl_registry::SemanticOperationId;
    use tcl_registry::hooks::LoweringHookId;

    if invocation.facts.operation != SemanticOperationId::StructuredLowering(LoweringHookId::Proc) {
        return ProcedureDefinitionReplay::NotProcedure;
    }
    match invocation.facts.arity_accepts_frozen_arguments() {
        Some(true) => {}
        Some(false) => return ProcedureDefinitionReplay::KnownError,
        None => return ProcedureDefinitionReplay::Unavailable,
    }
    if !invocation.facts.arg_roles_complete {
        return ProcedureDefinitionReplay::Unavailable;
    }
    let role_index = |role| {
        invocation
            .facts
            .arg_roles
            .iter()
            .find_map(|(index, found)| (*found == role).then_some(usize::from(*index)))
    };
    let (Some(params_index), Some(body_index)) = (
        role_index(tcl_registry::ArgRole::ParamList),
        role_index(tcl_registry::ArgRole::Body),
    ) else {
        return ProcedureDefinitionReplay::Unavailable;
    };
    let Some(name) = invocation
        .facts
        .state_transitions
        .declared()
        .and_then(|transitions| {
            transitions
                .command_bindings()
                .find_map(|transition| match transition {
                    CommandBindingTransition::Define { name, kind }
                        if *kind == CommandBindingDefinitionKind::Procedure =>
                    {
                        name.literal()
                    }
                    _ => None,
                })
        })
    else {
        return ProcedureDefinitionReplay::Unavailable;
    };
    let (Some(params), Some(body)) = (
        invocation
            .literal_arguments
            .get(params_index)
            .and_then(Option::as_deref),
        invocation
            .literal_arguments
            .get(body_index)
            .and_then(Option::as_deref),
    ) else {
        return ProcedureDefinitionReplay::Unavailable;
    };
    let Some(grammar) = dialect.and_then(tcl_registry::InvocationDialect::parameter_grammar) else {
        return ProcedureDefinitionReplay::Unavailable;
    };
    if tcl_syntax::formal_params::parse_formal_parameters_in(params, grammar).is_err() {
        return ProcedureDefinitionReplay::KnownError;
    }

    let Some(qname) = procedure_publication_key(namespace, name, dialect) else {
        return ProcedureDefinitionReplay::Unavailable;
    };
    let module = recovered_procedure_source(body, &qname);
    ProcedureDefinitionReplay::Recovered {
        qname,
        module: Box::new(module),
    }
}
