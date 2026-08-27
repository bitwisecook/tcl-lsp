// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! CFG-aware renaming of registry-owned mutable interpreter state.
//!
//! This is the world counterpart to cell SSA.  It consumes the target-neutral
//! executable IR, registry [`tcl_registry::InvocationFacts`], and completion
//! switches.  It deliberately has no command-name recognisers: a command,
//! namespace, trace, interpreter, or object transition arrives here only as a
//! registry [`tcl_registry::StateTransition`] fact.
//!
//! The builder is conservative in two important ways.  First, an invocation
//! with no closed registry transition declaration is a `Use(Any)` followed by
//! `Clobber(Any)`.  Secondly, a transition that may commit before an abrupt
//! completion is represented by a clobber-version on every abrupt edge.  That
//! version is the join of unchanged and changed state; the generic
//! [`crate::state_ssa::StateSsa`] substrate has one incoming value per CFG
//! predecessor, rather than a second hidden state representation.

use std::collections::{BTreeMap, BTreeSet};

use tcl_core_types::Code as CompletionCode;
use tcl_registry::{
    CommandBindingTransition, InterpreterTransition, NamespaceTransition,
    NamespaceTransitionTarget, ObjectDispatchTarget, ObjectDispatchTransition,
    ObjectPrivateNamespace, StateTransition, StateTransitionCommit, StateTransitionDomain,
    StateTransitionFact, StateTransitionKnowledge, TraceTarget, TraceTransition, TransitionSubject,
    VariableAliasTarget,
};

use crate::cfg::BlockId;
use crate::effect_ssa::{WorldStateIntentKind, project_effect_footprint};
use crate::executable_ir::{
    ExecutableBlockId, ExecutableFunction, ExecutableInstruction, ExecutableIrValidationError,
    ExecutableTerminator, InvocationResolution,
};
use crate::state_ssa::StateOverlap;
use crate::state_ssa::adapters::{
    WorldInterpreterScope, WorldNamespaceScope, WorldRegion, WorldRegionKind,
    WorldRegionOverlapPolicy, WorldSubjectScope,
};
use crate::state_ssa::{
    CfgStatePosition, StateClobber, StateDef, StateOp, StatePhi, StateSite, StateSsa,
    StateSsaError, StateUse, StateVersion,
};

/// One version-free transition operation, in descriptor order.
///
/// It intentionally keeps the registry's completion commit policy.  The CFG
/// builder decides which outgoing completion edges receive the operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionStateIntent {
    /// The transition's state operation.
    pub kind: WorldStateIntentKind,
    /// The mutable interpreter-world region affected by the transition.
    pub location: WorldRegion,
    /// Completion-edge commit semantics declared by the registry.
    pub commit: StateTransitionCommit,
}

type InvocationIntents = (
    Vec<(WorldStateIntentKind, WorldRegion)>,
    Vec<TransitionStateIntent>,
);

/// Project closed registry transition facts into version-free state intents.
///
/// Dynamic transition operands become typed domain wildcards.  This is the
/// only transition vocabulary common passes need; command spellings remain in
/// the registry resolver that created the facts.
#[must_use]
pub fn project_transition_facts(facts: &[StateTransitionFact]) -> Vec<TransitionStateIntent> {
    let mut intents = Vec::new();
    for fact in facts {
        append_transition(&mut intents, fact);
    }
    intents
}

fn project_invocation_intents(
    resolution: &InvocationResolution,
    state_site: &StateSite,
    block: ExecutableBlockId,
    instruction: usize,
) -> Result<InvocationIntents, WorldStateSsaDecline> {
    let mut ordinary = Vec::new();
    let mut transition = Vec::new();
    match resolution {
        InvocationResolution::Resolved(facts) => {
            ordinary.extend(
                project_effect_footprint(facts.world_state_effects(), state_site, None)
                    .map_err(|_| WorldStateSsaDecline::StateSiteOverflow { block, instruction })?
                    .into_iter()
                    .map(|intent| (intent.kind, intent.location)),
            );
            match &facts.state_transitions {
                StateTransitionKnowledge::Declared(transitions) => {
                    transition.extend(project_transition_facts(transitions.facts()));
                }
                StateTransitionKnowledge::UnknownInvocation => {
                    ordinary.push((WorldStateIntentKind::Use, WorldRegion::Any));
                    ordinary.push((WorldStateIntentKind::Clobber, WorldRegion::Any));
                }
            }
        }
        InvocationResolution::Unresolved(_) => {
            // A computed head and an unknown literal command both run Tcl's
            // resolver, `unknown`, aliases, and traces.
            ordinary.push((WorldStateIntentKind::Use, WorldRegion::Any));
            ordinary.push((WorldStateIntentKind::Clobber, WorldRegion::Any));
        }
    }
    Ok((ordinary, transition))
}

fn append_transition(intents: &mut Vec<TransitionStateIntent>, fact: &StateTransitionFact) {
    match &fact.transition {
        StateTransition::CommandBinding(transition) => {
            append_command_binding(intents, fact.commit, transition);
        }
        StateTransition::Interpreter(transition) => {
            append_interpreter_transition(intents, fact.commit, transition);
        }
        StateTransition::VariableCellAlias(alias) => {
            append_variable_alias(intents, fact.commit, alias);
        }
        StateTransition::Namespace(transition) => {
            append_namespace(intents, fact.commit, transition);
        }
        StateTransition::Trace(transition) => append_trace(intents, fact.commit, transition),
        StateTransition::ObjectDispatch(transition) => {
            append_object_dispatch(intents, fact.commit, transition);
        }
        StateTransition::Widen(widening) => append_widening(intents, fact.commit, widening),
    }
}

fn append_object_dispatch(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    transition: &ObjectDispatchTransition,
) {
    match transition {
        ObjectDispatchTransition::Configure { target, layer: _ } => {
            append_object_configuration(intents, commit, target);
        }
        ObjectDispatchTransition::Create {
            target,
            private_namespace,
            kind: _,
        } => append_object_creation(intents, commit, target, private_namespace),
        ObjectDispatchTransition::Copy {
            source,
            target,
            private_namespace,
        } => append_object_copy(intents, commit, source, target, private_namespace),
        ObjectDispatchTransition::Destroy { target } => {
            append_object_destruction(intents, commit, target);
        }
    }
}

fn append_object_configuration(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    target: &TransitionSubject,
) {
    // The registry retains class-wide versus per-object configuration for LSP
    // and future dispatch-chain reasoning. This first common state partition
    // aliases both layers deliberately: proving them disjoint would require
    // the class/object ownership graph, which executable IR does not yet carry.
    let region = object_dispatch_region(&ObjectDispatchTarget::Named(target.clone()));
    push_intent(intents, commit, WorldStateIntentKind::Use, region.clone());
    push_intent(intents, commit, WorldStateIntentKind::Def, region);
}

fn append_object_creation(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    target: &ObjectDispatchTarget,
    private_namespace: &ObjectPrivateNamespace,
) {
    push_intent(
        intents,
        commit,
        WorldStateIntentKind::Def,
        object_dispatch_region(target),
    );
    append_object_private_state_write(intents, commit, private_namespace);
}

fn append_object_copy(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    source: &TransitionSubject,
    target: &ObjectDispatchTarget,
    private_namespace: &ObjectPrivateNamespace,
) {
    push_intent(
        intents,
        commit,
        WorldStateIntentKind::Use,
        object_dispatch_region(&ObjectDispatchTarget::Named(source.clone())),
    );
    // Object command names and private namespace identities are deliberately
    // distinct in TclOO. Until dispatch state carries the reverse mapping,
    // copying reads an unknown private variable partition rather than deriving
    // one from `source`'s spelling.
    push_intent(
        intents,
        commit,
        WorldStateIntentKind::Use,
        unknown_current_region(WorldRegionKind::VariableStore),
    );
    push_intent(
        intents,
        commit,
        WorldStateIntentKind::Def,
        object_dispatch_region(target),
    );
    append_object_private_state_write(intents, commit, private_namespace);
}

fn append_object_private_state_write(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    private_namespace: &ObjectPrivateNamespace,
) {
    push_intent(
        intents,
        commit,
        WorldStateIntentKind::Def,
        object_private_namespace_region(private_namespace),
    );
    push_intent(
        intents,
        commit,
        WorldStateIntentKind::Clobber,
        object_private_variable_region(private_namespace),
    );
}

fn append_object_destruction(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    target: &TransitionSubject,
) {
    let dispatch = object_dispatch_region(&ObjectDispatchTarget::Named(target.clone()));
    push_intent(intents, commit, WorldStateIntentKind::Use, dispatch.clone());
    push_intent(intents, commit, WorldStateIntentKind::Def, dispatch);
    // The private namespace identity is owned by the object rather than
    // derived from its public command spelling. Until dispatch state carries
    // that reverse mapping, destruction invalidates both private partitions.
    for kind in [
        WorldRegionKind::NamespaceLookup,
        WorldRegionKind::VariableStore,
    ] {
        push_intent(
            intents,
            commit,
            WorldStateIntentKind::Clobber,
            unknown_current_region(kind),
        );
    }
}

fn unknown_current_region(kind: WorldRegionKind) -> WorldRegion {
    WorldRegion::domain_wildcard(
        kind,
        WorldInterpreterScope::Current,
        WorldNamespaceScope::Any,
    )
}

fn push_intent(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    kind: WorldStateIntentKind,
    location: WorldRegion,
) {
    intents.push(TransitionStateIntent {
        kind,
        location,
        commit,
    });
}

fn append_command_binding(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    transition: &CommandBindingTransition,
) {
    match transition {
        CommandBindingTransition::Define { name, kind: _ } => push_intent(
            intents,
            commit,
            WorldStateIntentKind::Def,
            command_region(WorldInterpreterScope::Current, name),
        ),
        CommandBindingTransition::Move { from, to } => {
            let interpreter = WorldInterpreterScope::Current;
            push_intent(
                intents,
                commit,
                WorldStateIntentKind::Use,
                command_region(interpreter.clone(), from),
            );
            push_intent(
                intents,
                commit,
                WorldStateIntentKind::Def,
                command_region(interpreter.clone(), from),
            );
            push_intent(
                intents,
                commit,
                WorldStateIntentKind::Def,
                command_region(interpreter, to),
            );
        }
        CommandBindingTransition::Delete { interpreter, name } => {
            let interpreter = interpreter
                .as_ref()
                .map_or(WorldInterpreterScope::Current, subject_interpreter);
            push_intent(
                intents,
                commit,
                WorldStateIntentKind::Def,
                command_region(interpreter, name),
            );
        }
        CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            target_interpreter,
            target,
        } => {
            let source = subject_interpreter(source_interpreter);
            let target_interpreter = subject_interpreter(target_interpreter);
            push_intent(
                intents,
                commit,
                WorldStateIntentKind::Use,
                command_region(target_interpreter, target),
            );
            push_intent(
                intents,
                commit,
                WorldStateIntentKind::Def,
                command_region(source, alias),
            );
        }
        CommandBindingTransition::Unknown { .. } => push_intent(
            intents,
            commit,
            WorldStateIntentKind::Clobber,
            WorldRegion::domain_wildcard(
                WorldRegionKind::CommandBindings,
                WorldInterpreterScope::Any,
                WorldNamespaceScope::Any,
            ),
        ),
    }
}

fn append_interpreter_transition(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    transition: &InterpreterTransition,
) {
    match transition {
        InterpreterTransition::Create {
            interpreter,
            safety: _,
        } => append_interpreter_creation(intents, commit, interpreter.as_ref()),
        InterpreterTransition::Delete { interpreter } => {
            append_interpreter_deletion(intents, commit, interpreter);
        }
        InterpreterTransition::MarkTrusted { interpreter }
        | InterpreterTransition::SetRecursionLimit { interpreter, .. }
        | InterpreterTransition::SetBackgroundError { interpreter, .. } => {
            append_interpreter_policy_update(intents, commit, interpreter);
        }
        InterpreterTransition::Hide {
            interpreter,
            visible,
            hidden: _,
        } => append_interpreter_hide(intents, commit, interpreter, visible),
        InterpreterTransition::Expose {
            interpreter,
            hidden: _,
            visible,
        } => append_interpreter_expose(intents, commit, interpreter, visible),
    }
}

fn append_interpreter_creation(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    interpreter: Option<&TransitionSubject>,
) {
    let scope = optional_subject_interpreter(interpreter);
    // A child at a reused path is a fresh interpreter. Give every child-owned
    // region a new version instead of inheriting an earlier child's state.
    push_intent(
        intents,
        commit,
        WorldStateIntentKind::Def,
        WorldRegion::InterpreterWildcard {
            interpreter: scope.clone(),
        },
    );
    push_intent(
        intents,
        commit,
        WorldStateIntentKind::Def,
        interpreter_region(WorldRegionKind::InterpreterTopology, scope.clone()),
    );
    push_intent(
        intents,
        commit,
        WorldStateIntentKind::Def,
        interpreter_region(WorldRegionKind::InterpreterPolicy, scope),
    );
    if let Some(interpreter) = interpreter {
        push_intent(
            intents,
            commit,
            WorldStateIntentKind::Def,
            command_region(WorldInterpreterScope::Current, interpreter),
        );
    }
}

fn append_interpreter_deletion(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    interpreter: &TransitionSubject,
) {
    let scope = subject_interpreter(interpreter);
    push_intent(
        intents,
        commit,
        WorldStateIntentKind::Use,
        interpreter_region(WorldRegionKind::InterpreterTopology, scope.clone()),
    );
    push_intent(
        intents,
        commit,
        WorldStateIntentKind::Def,
        WorldRegion::InterpreterWildcard { interpreter: scope },
    );
    // Any surviving interpreter can retain an alias whose target was the
    // deleted child. Reverse alias edges are not yet a state topology.
    push_intent(
        intents,
        commit,
        WorldStateIntentKind::Clobber,
        WorldRegion::domain_wildcard(
            WorldRegionKind::CommandBindings,
            WorldInterpreterScope::Any,
            WorldNamespaceScope::Any,
        ),
    );
}

fn append_interpreter_policy_update(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    interpreter: &TransitionSubject,
) {
    let scope = subject_interpreter(interpreter);
    push_interpreter_topology_use(intents, commit, scope.clone());
    push_intent(
        intents,
        commit,
        WorldStateIntentKind::Def,
        interpreter_region(WorldRegionKind::InterpreterPolicy, scope),
    );
}

fn append_interpreter_hide(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    interpreter: &TransitionSubject,
    visible: &TransitionSubject,
) {
    let scope = subject_interpreter(interpreter);
    push_interpreter_topology_use(intents, commit, scope.clone());
    let command = command_region(scope.clone(), visible);
    push_intent(intents, commit, WorldStateIntentKind::Use, command.clone());
    push_intent(intents, commit, WorldStateIntentKind::Def, command);
    push_intent(
        intents,
        commit,
        WorldStateIntentKind::Def,
        interpreter_region(WorldRegionKind::InterpreterPolicy, scope),
    );
}

fn append_interpreter_expose(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    interpreter: &TransitionSubject,
    visible: &TransitionSubject,
) {
    let scope = subject_interpreter(interpreter);
    push_interpreter_topology_use(intents, commit, scope.clone());
    push_intent(
        intents,
        commit,
        WorldStateIntentKind::Def,
        command_region(scope.clone(), visible),
    );
    push_intent(
        intents,
        commit,
        WorldStateIntentKind::Def,
        interpreter_region(WorldRegionKind::InterpreterPolicy, scope),
    );
}

fn push_interpreter_topology_use(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    scope: WorldInterpreterScope,
) {
    push_intent(
        intents,
        commit,
        WorldStateIntentKind::Use,
        interpreter_region(WorldRegionKind::InterpreterTopology, scope),
    );
}

fn append_variable_alias(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    alias: &tcl_registry::VariableCellAliasTransition,
) {
    // Variable-cell identity is world state here.  It is explicitly not a
    // `Place`: cell SSA owns concrete local storage separately.
    let target = match &alias.target {
        VariableAliasTarget::Global { variable }
        | VariableAliasTarget::CurrentNamespace { variable } => {
            variable_region(WorldInterpreterScope::Current, variable)
        }
        VariableAliasTarget::CallerSelectedFrame { variable, .. } => {
            variable_region(WorldInterpreterScope::Any, variable)
        }
        VariableAliasTarget::Namespace {
            namespace,
            variable,
        } => WorldRegion::scoped(
            WorldRegionKind::VariableStore,
            WorldInterpreterScope::Current,
            subject_namespace(namespace),
            subject_scope(variable),
        ),
    };
    push_intent(intents, commit, WorldStateIntentKind::Use, target);
    push_intent(
        intents,
        commit,
        WorldStateIntentKind::Def,
        variable_region(WorldInterpreterScope::Current, &alias.local),
    );
}

fn append_namespace(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    transition: &NamespaceTransition,
) {
    match transition {
        NamespaceTransition::Ensure { namespace } => push_intent(
            intents,
            commit,
            WorldStateIntentKind::Def,
            WorldRegion::namespace_lineage(
                WorldInterpreterScope::Current,
                namespace_target(namespace),
            ),
        ),
        NamespaceTransition::Delete { namespace } => push_intent(
            intents,
            commit,
            WorldStateIntentKind::Def,
            WorldRegion::namespace_subtree(
                WorldInterpreterScope::Current,
                namespace_target(namespace),
            ),
        ),
        NamespaceTransition::Export {
            namespace,
            patterns: _,
        }
        | NamespaceTransition::SetPath { namespace, path: _ } => push_intent(
            intents,
            commit,
            WorldStateIntentKind::Def,
            namespace_region(WorldRegionKind::NamespaceLookup, namespace),
        ),
        NamespaceTransition::Import {
            namespace,
            patterns: _,
        }
        | NamespaceTransition::Forget {
            namespace,
            patterns: _,
        }
        | NamespaceTransition::Ensemble { namespace } => {
            let namespace = namespace_target(namespace);
            append_namespace_bindings(intents, commit, &namespace);
        }
        NamespaceTransition::SetUnknown {
            namespace,
            handler: _,
        } => push_intent(
            intents,
            commit,
            WorldStateIntentKind::Def,
            namespace_region(WorldRegionKind::NamespaceUnknown, namespace),
        ),
    }
}

fn append_namespace_bindings(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    namespace: &WorldNamespaceScope,
) {
    for kind in [
        WorldRegionKind::CommandBindings,
        WorldRegionKind::NamespaceLookup,
    ] {
        push_intent(
            intents,
            commit,
            WorldStateIntentKind::Def,
            WorldRegion::domain_wildcard(kind, WorldInterpreterScope::Current, namespace.clone()),
        );
    }
}

fn append_trace(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    transition: &TraceTransition,
) {
    match transition {
        TraceTransition::Add { target, .. } => append_trace_add(intents, commit, target),
        TraceTransition::Remove { target, .. } => append_trace_remove(intents, commit, target),
    }
}

fn append_trace_add(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    target: &TraceTarget,
) {
    match target {
        TraceTarget::Variable(target) => {
            // `trace add variable` creates an otherwise absent cell before registering its trace.
            append_read_write(
                intents,
                commit,
                variable_region(WorldInterpreterScope::Current, target),
            );
            append_read_write(
                intents,
                commit,
                trace_region(WorldRegionKind::VariableTraces, target),
            );
        }
        TraceTarget::Command(target) => {
            push_intent(
                intents,
                commit,
                WorldStateIntentKind::Use,
                command_region(WorldInterpreterScope::Current, target),
            );
            append_read_write(
                intents,
                commit,
                trace_region(WorldRegionKind::CommandTraces, target),
            );
        }
        TraceTarget::Execution(target) => {
            push_intent(
                intents,
                commit,
                WorldStateIntentKind::Use,
                command_region(WorldInterpreterScope::Current, target),
            );
            append_read_write(
                intents,
                commit,
                trace_region(WorldRegionKind::ExecutionTraces, target),
            );
        }
    }
}

fn append_trace_remove(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    target: &TraceTarget,
) {
    let (trace_kind, target) = match target {
        TraceTarget::Variable(target) => {
            // Removing the final trace from an otherwise undefined variable
            // also releases the placeholder cell created by trace
            // registration. That is observable through `namespace which
            // -variable`, even though removing a trace from a completely
            // missing variable is a no-op.
            append_read_write(
                intents,
                commit,
                variable_region(WorldInterpreterScope::Current, target),
            );
            (WorldRegionKind::VariableTraces, target)
        }
        TraceTarget::Command(target) => {
            push_intent(
                intents,
                commit,
                WorldStateIntentKind::Use,
                command_region(WorldInterpreterScope::Current, target),
            );
            (WorldRegionKind::CommandTraces, target)
        }
        TraceTarget::Execution(target) => {
            push_intent(
                intents,
                commit,
                WorldStateIntentKind::Use,
                command_region(WorldInterpreterScope::Current, target),
            );
            (WorldRegionKind::ExecutionTraces, target)
        }
    };
    append_read_write(intents, commit, trace_region(trace_kind, target));
}

fn append_read_write(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    location: WorldRegion,
) {
    push_intent(intents, commit, WorldStateIntentKind::Use, location.clone());
    push_intent(intents, commit, WorldStateIntentKind::Def, location);
}

fn append_widening(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    widening: &tcl_registry::StateTransitionWidening,
) {
    for domain in &widening.domains {
        for location in domain_wildcards(*domain) {
            push_intent(intents, commit, WorldStateIntentKind::Clobber, location);
        }
    }
}

fn namespace_target(target: &NamespaceTransitionTarget) -> WorldNamespaceScope {
    match target {
        NamespaceTransitionTarget::Current => WorldNamespaceScope::Current,
        // An unqualified Tcl namespace word is rooted in the dynamic current
        // namespace.  Retaining its spelling as a globally stable name would
        // prove unrelated trees disjoint, so only fully-qualified literals
        // become exact scopes here.
        NamespaceTransitionTarget::Named(TransitionSubject::Literal(namespace))
            if namespace.starts_with("::") =>
        {
            WorldNamespaceScope::named(namespace)
        }
        NamespaceTransitionTarget::Named(_) => WorldNamespaceScope::Any,
    }
}

fn namespace_region(kind: WorldRegionKind, namespace: &NamespaceTransitionTarget) -> WorldRegion {
    WorldRegion::domain_wildcard(
        kind,
        WorldInterpreterScope::Current,
        namespace_target(namespace),
    )
}

fn command_region(interpreter: WorldInterpreterScope, subject: &TransitionSubject) -> WorldRegion {
    WorldRegion::scoped(
        WorldRegionKind::CommandBindings,
        interpreter,
        WorldNamespaceScope::Current,
        subject_scope(subject),
    )
}

fn interpreter_region(kind: WorldRegionKind, interpreter: WorldInterpreterScope) -> WorldRegion {
    WorldRegion::domain_wildcard(kind, interpreter, WorldNamespaceScope::Any)
}

fn variable_region(interpreter: WorldInterpreterScope, subject: &TransitionSubject) -> WorldRegion {
    WorldRegion::scoped(
        WorldRegionKind::VariableStore,
        interpreter,
        WorldNamespaceScope::Current,
        subject_scope(subject),
    )
}

fn trace_region(kind: WorldRegionKind, subject: &TransitionSubject) -> WorldRegion {
    WorldRegion::scoped(
        kind,
        WorldInterpreterScope::Current,
        WorldNamespaceScope::Current,
        subject_scope(subject),
    )
}

fn object_dispatch_region(target: &ObjectDispatchTarget) -> WorldRegion {
    let subject = match target {
        ObjectDispatchTarget::Named(subject) => subject_scope(subject),
        // A generated object is fresh, but the invocation facts do not yet
        // carry a CFG-site identity.  Retain the lifecycle transition while
        // conservatively overlapping every object until that identity is
        // threaded through executable IR.
        ObjectDispatchTarget::Fresh => WorldSubjectScope::Wildcard,
    };
    WorldRegion::scoped(
        WorldRegionKind::ObjectDispatch,
        WorldInterpreterScope::Current,
        WorldNamespaceScope::Current,
        subject,
    )
}

fn object_private_namespace_region(namespace: &ObjectPrivateNamespace) -> WorldRegion {
    let namespace = match namespace {
        ObjectPrivateNamespace::Named(subject) => subject_namespace(subject),
        ObjectPrivateNamespace::Fresh => WorldNamespaceScope::Any,
    };
    WorldRegion::domain_wildcard(
        WorldRegionKind::NamespaceLookup,
        WorldInterpreterScope::Current,
        namespace,
    )
}

fn object_private_variable_region(namespace: &ObjectPrivateNamespace) -> WorldRegion {
    let namespace = match namespace {
        ObjectPrivateNamespace::Named(subject) => subject_namespace(subject),
        ObjectPrivateNamespace::Fresh => WorldNamespaceScope::Any,
    };
    WorldRegion::domain_wildcard(
        WorldRegionKind::VariableStore,
        WorldInterpreterScope::Current,
        namespace,
    )
}

fn subject_scope(subject: &TransitionSubject) -> WorldSubjectScope {
    match subject {
        TransitionSubject::Literal(value) => WorldSubjectScope::named(value),
        TransitionSubject::Unknown { .. } => WorldSubjectScope::Wildcard,
    }
}

fn subject_interpreter(subject: &TransitionSubject) -> WorldInterpreterScope {
    match subject {
        TransitionSubject::Literal(value) => WorldInterpreterScope::named(value),
        TransitionSubject::Unknown { .. } => WorldInterpreterScope::Any,
    }
}

fn optional_subject_interpreter(subject: Option<&TransitionSubject>) -> WorldInterpreterScope {
    subject.map_or(WorldInterpreterScope::Any, subject_interpreter)
}

fn subject_namespace(subject: &TransitionSubject) -> WorldNamespaceScope {
    match subject {
        TransitionSubject::Literal(value) => WorldNamespaceScope::named(value),
        TransitionSubject::Unknown { .. } => WorldNamespaceScope::Any,
    }
}

fn domain_wildcards(domain: StateTransitionDomain) -> Vec<WorldRegion> {
    let scopes = || (WorldInterpreterScope::Any, WorldNamespaceScope::Any);
    let region = |kind| {
        let (interpreter, namespace) = scopes();
        WorldRegion::domain_wildcard(kind, interpreter, namespace)
    };
    match domain {
        StateTransitionDomain::CommandBindings => vec![region(WorldRegionKind::CommandBindings)],
        StateTransitionDomain::VariableCells => vec![region(WorldRegionKind::VariableStore)],
        // Namespace membership/lookup and namespace-specific `unknown`
        // handlers are distinct partitions, so a dynamic namespace operand
        // must invalidate both.
        StateTransitionDomain::Namespaces => vec![
            region(WorldRegionKind::NamespaceLookup),
            region(WorldRegionKind::NamespaceUnknown),
        ],
        StateTransitionDomain::InterpreterTopology => {
            vec![region(WorldRegionKind::InterpreterTopology)]
        }
        StateTransitionDomain::InterpreterPolicy => {
            vec![region(WorldRegionKind::InterpreterPolicy)]
        }
        // A dynamic interpreter path can expose any partition owned by an
        // interpreter. `InterpreterWildcard` is intentionally wider than a
        // policy-only region.
        StateTransitionDomain::Interpreters => vec![WorldRegion::InterpreterWildcard {
            interpreter: WorldInterpreterScope::Any,
        }],
        StateTransitionDomain::CommandTraces => vec![region(WorldRegionKind::CommandTraces)],
        StateTransitionDomain::ExecutionTraces => vec![region(WorldRegionKind::ExecutionTraces)],
        StateTransitionDomain::VariableTraces => vec![region(WorldRegionKind::VariableTraces)],
        StateTransitionDomain::ObjectDispatch => vec![region(WorldRegionKind::ObjectDispatch)],
    }
}

/// Why a world-state build cannot safely interpret an executable CFG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldStateSsaDecline {
    /// The executable semantic CFG failed its own target-neutral validation.
    InvalidExecutableIr(ExecutableIrValidationError),
    /// A block index cannot be represented by the shared CFG [`BlockId`].
    BlockIdOverflow {
        /// Executable block index that did not fit the shared identifier.
        block: usize,
    },
    /// A CFG instruction or its ordered state operations cannot fit the
    /// shared `u32` state-site coordinate space.
    StateSiteOverflow {
        /// Containing executable block.
        block: ExecutableBlockId,
        /// Instruction position in that block.
        instruction: usize,
    },
    /// A transition-bearing invocation must end in a completion dispatch.
    MissingCompletionSwitch {
        /// Transition-bearing block without a completion switch.
        block: ExecutableBlockId,
    },
    /// The completion switch did not dispatch the invocation completion.
    CompletionSwitchMismatch {
        /// Block whose switch dispatches a different completion.
        block: ExecutableBlockId,
    },
    /// The completion switch does not explicitly retain the `TCL_OK` edge.
    MissingOkCompletionEdge {
        /// Block whose completion switch has no explicit `TCL_OK` arm.
        block: ExecutableBlockId,
    },
    /// A validated CFG edge was absent from the planner's deterministic
    /// successor table.
    MissingCfgEdge {
        /// Block selecting the missing edge.
        predecessor: usize,
        /// Target block absent from the predecessor's successor table.
        successor: usize,
    },
    /// A reachable non-entry block had no state predecessor during renaming.
    MissingStatePredecessor {
        /// Reachable block without a state predecessor.
        block: usize,
        /// State-location slot being renamed.
        slot: usize,
    },
    /// A state symbol which reaches a record was not assigned a version.
    MissingStateVersion,
    /// A state-SSA record could not be validated.
    InvalidStateSsa(StateSsaError),
}

/// A built world-state SSA result and the typed region-overlap policy used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableWorldStateSsa {
    /// Validated common state-SSA records.
    pub state: StateSsa<WorldRegion>,
    /// Locations tracked by the renamer, in deterministic discovery order.
    pub locations: Vec<WorldRegion>,
}

/// Build world-state SSA for one executable function.
///
/// The result is deterministic for an isomorphic CFG irrespective of
/// predecessor insertion order.  It computes the fixed point over reachable
/// blocks, including back edges, before materialising state phi records.
pub fn build_executable_world_state_ssa(
    function: &ExecutableFunction,
) -> Result<ExecutableWorldStateSsa, WorldStateSsaDecline> {
    function
        .validate()
        .map_err(WorldStateSsaDecline::InvalidExecutableIr)?;
    let mut planner = Planner::new(function)?;
    planner.plan()?;
    planner.rename()
}

#[derive(Debug, Clone)]
struct Token {
    kind: WorldStateIntentKind,
    location: usize,
    site: StateSite,
}

#[derive(Debug, Clone)]
struct TransitionToken {
    /// Edge-qualified exact or conservative join definition tokens.
    edge_tokens: BTreeMap<usize, usize>,
}

#[derive(Debug, Default, Clone)]
struct BlockPlan {
    base: Vec<usize>,
    transitions: Vec<TransitionToken>,
    transition_completion: Option<crate::executable_ir::CompletionId>,
}

struct Planner<'a> {
    function: &'a ExecutableFunction,
    locations: Vec<WorldRegion>,
    tokens: Vec<Token>,
    plans: Vec<BlockPlan>,
    successors: Vec<Vec<usize>>,
    edge_completion: Vec<Vec<EdgeCompletion>>,
    predecessors: Vec<Vec<usize>>,
    reachable: BTreeSet<usize>,
}

impl<'a> Planner<'a> {
    fn new(function: &'a ExecutableFunction) -> Result<Self, WorldStateSsaDecline> {
        for index in 0..function.blocks.len() {
            let _ = block_id(index)?;
        }
        let mut successors = vec![Vec::new(); function.blocks.len()];
        let mut predecessors = vec![Vec::new(); function.blocks.len()];
        let mut edge_completions = vec![Vec::new(); function.blocks.len()];
        for (index, block) in function.blocks.iter().enumerate() {
            if let Some(terminator) = &block.terminator {
                successors[index] = successors_of(terminator);
                successors[index].sort_unstable();
                successors[index].dedup();
                edge_completions[index] = successors[index]
                    .iter()
                    .map(|successor| edge_completion(terminator, *successor))
                    .collect();
                for successor in &successors[index] {
                    if *successor < predecessors.len() {
                        predecessors[*successor].push(index);
                    }
                }
            }
        }
        for predecessors in &mut predecessors {
            predecessors.sort_unstable();
            predecessors.dedup();
        }
        let mut reachable = BTreeSet::new();
        let mut pending = vec![function.entry.index()];
        while let Some(block) = pending.pop() {
            if block >= successors.len() || !reachable.insert(block) {
                continue;
            }
            pending.extend(successors[block].iter().rev().copied());
        }
        Ok(Self {
            function,
            locations: Vec::new(),
            tokens: Vec::new(),
            plans: vec![BlockPlan::default(); function.blocks.len()],
            successors,
            edge_completion: edge_completions,
            predecessors,
            reachable,
        })
    }

    fn plan(&mut self) -> Result<(), WorldStateSsaDecline> {
        for block_index in 0..self.function.blocks.len() {
            // Unreachable semantic actions must not allocate locations,
            // versions, or impose completion-shape requirements on the live
            // CFG.  Besides keeping diagnostics focused on executable code,
            // this ensures a dead transition cannot perturb the deterministic
            // numbering of the reachable state graph.
            if !self.reachable.contains(&block_index) {
                continue;
            }
            let mut ordinal = 0_u32;
            for instruction_index in 0..self.function.blocks[block_index].instructions.len() {
                let plan_inputs = {
                    let block = &self.function.blocks[block_index];
                    match &block.instructions[instruction_index] {
                        ExecutableInstruction::Invoke(invoke) => {
                            let state_site =
                                Self::state_site(block.id, instruction_index, ordinal)?;
                            project_invocation_intents(
                                &invoke.resolution,
                                &state_site,
                                block.id,
                                instruction_index,
                            )
                            .map(|(ordinary, transition)| {
                                Some((block.id, invoke.completion, ordinary, transition))
                            })
                        }
                        ExecutableInstruction::ExecuteLowered(operation) => Ok(Some((
                            block.id,
                            operation.completion,
                            conservative_region_intents(),
                            Vec::new(),
                        ))),
                        ExecutableInstruction::ExecuteOpaqueRegion(region) => Ok(Some((
                            block.id,
                            region.completion,
                            conservative_region_intents(),
                            Vec::new(),
                        ))),
                        ExecutableInstruction::EvaluateWord {
                            word, completion, ..
                        } if word_evaluation_runs_commands(word) => {
                            // A word containing a command substitution (or an
                            // opaque fragment that may hide one) runs arbitrary
                            // commands during argv evaluation.  Without this
                            // barrier a closed outer invocation such as a pure
                            // list query would hide a nested `rename` from the
                            // world graph.
                            Ok(Some((
                                block.id,
                                *completion,
                                conservative_region_intents(),
                                Vec::new(),
                            )))
                        }
                        _ => Ok(None),
                    }
                }?;
                let Some((block, completion, ordinary, transition)) = plan_inputs else {
                    continue;
                };
                for (kind, location) in ordinary {
                    let token = self.token(
                        kind,
                        location,
                        Self::state_site(block, instruction_index, ordinal)?,
                    );
                    self.plans[block_index].base.push(token);
                    ordinal = next_ordinal(block, instruction_index, ordinal)?;
                }
                self.plan_transition_intents(
                    block_index,
                    block,
                    instruction_index,
                    &mut ordinal,
                    completion,
                    transition,
                )?;
            }
        }
        Ok(())
    }

    fn plan_transition_intents(
        &mut self,
        block_index: usize,
        block: ExecutableBlockId,
        instruction: usize,
        ordinal: &mut u32,
        invocation_completion: crate::executable_ir::CompletionId,
        transitions: Vec<TransitionStateIntent>,
    ) -> Result<(), WorldStateSsaDecline> {
        if transitions.is_empty() {
            return Ok(());
        }
        let Some(ExecutableTerminator::CompletionSwitch {
            completion, cases, ..
        }) = self.function.blocks[block_index].terminator.as_ref()
        else {
            return Err(WorldStateSsaDecline::MissingCompletionSwitch { block });
        };
        if *completion != invocation_completion {
            return Err(WorldStateSsaDecline::CompletionSwitchMismatch { block });
        }
        if !cases.iter().any(|case| case.code == CompletionCode::Ok) {
            return Err(WorldStateSsaDecline::MissingOkCompletionEdge { block });
        }
        if self.plans[block_index].transition_completion.is_some() {
            return Err(WorldStateSsaDecline::CompletionSwitchMismatch { block });
        }
        self.plans[block_index].transition_completion = Some(*completion);
        for intent in transitions {
            self.plan_transition_intent(block_index, block, instruction, ordinal, intent)?;
        }
        Ok(())
    }

    fn plan_transition_intent(
        &mut self,
        block_index: usize,
        block: ExecutableBlockId,
        instruction: usize,
        ordinal: &mut u32,
        intent: TransitionStateIntent,
    ) -> Result<(), WorldStateSsaDecline> {
        if intent.kind == WorldStateIntentKind::Use {
            // A transition read is an execution-time dependency, independent
            // of completion outcome.
            let token = self.token(
                intent.kind,
                intent.location,
                Self::state_site(block, instruction, *ordinal)?,
            );
            self.plans[block_index].base.push(token);
            *ordinal = next_ordinal(block, instruction, *ordinal)?;
            return Ok(());
        }
        let mut edge_tokens = BTreeMap::new();
        for successor in self.successors[block_index].clone() {
            let mode = self.edge_mode(block_index, successor)?;
            let kind = transition_edge_kind(intent.kind, intent.commit, mode);
            let Some(kind) = kind else { continue };
            let token = self.token(
                kind,
                intent.location.clone(),
                Self::edge_site(block, successor, instruction, *ordinal)?,
            );
            *ordinal = next_ordinal(block, instruction, *ordinal)?;
            edge_tokens.insert(successor, token);
        }
        if !edge_tokens.is_empty() {
            self.plans[block_index]
                .transitions
                .push(TransitionToken { edge_tokens });
        }
        Ok(())
    }

    fn state_site(
        block: ExecutableBlockId,
        instruction: usize,
        ordinal: u32,
    ) -> Result<StateSite, WorldStateSsaDecline> {
        let instruction = u32::try_from(instruction)
            .map_err(|_| WorldStateSsaDecline::StateSiteOverflow { block, instruction })?;
        Ok(StateSite::statement(
            block_id(block.index())?,
            instruction,
            ordinal,
        ))
    }

    fn edge_site(
        predecessor: ExecutableBlockId,
        successor: usize,
        instruction: usize,
        ordinal: u32,
    ) -> Result<StateSite, WorldStateSsaDecline> {
        let instruction =
            u32::try_from(instruction).map_err(|_| WorldStateSsaDecline::StateSiteOverflow {
                block: predecessor,
                instruction,
            })?;
        Ok(StateSite::edge(
            block_id(predecessor.index())?,
            block_id(successor)?,
            CfgStatePosition::Statement {
                index: instruction,
                ordinal,
            },
        ))
    }

    fn token(
        &mut self,
        kind: WorldStateIntentKind,
        location: WorldRegion,
        site: StateSite,
    ) -> usize {
        let location = self.location(location);
        let index = self.tokens.len();
        self.tokens.push(Token {
            kind,
            location,
            site,
        });
        index
    }

    fn location(&mut self, location: WorldRegion) -> usize {
        if let Some(index) = self.locations.iter().position(|known| known == &location) {
            index
        } else {
            let index = self.locations.len();
            self.locations.push(location);
            index
        }
    }

    fn rename(self) -> Result<ExecutableWorldStateSsa, WorldStateSsaDecline> {
        let count = self.function.blocks.len();
        let slots = self.locations.len();
        let initial = vec![Symbol::Initial; slots];
        let mut entry = vec![initial.clone(); count];
        let mut edge_exit: Vec<Vec<Vec<Symbol>>> = self
            .successors
            .iter()
            .map(|successors| vec![initial.clone(); successors.len()])
            .collect();
        let entry_index = self.function.entry.index();

        loop {
            let mut changed = false;
            for block in self.reachable.iter().copied() {
                let next_entry = self.join_entry(block, &edge_exit, slots, block == entry_index)?;
                let next_edges = self.apply_block_edges(block, next_entry.clone());
                if entry[block] != next_entry || edge_exit[block] != next_edges {
                    entry[block] = next_entry;
                    edge_exit[block] = next_edges;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        let mut phi_symbols = BTreeSet::new();
        for (block, values) in entry.iter().enumerate() {
            if self.reachable.contains(&block) {
                phi_symbols.extend(values.iter().copied().filter_map(Symbol::as_phi));
            }
        }
        let mut versions = BTreeMap::new();
        let mut next = 1_u32;
        for symbol in phi_symbols {
            versions.insert(Symbol::Phi(symbol.0, symbol.1), take_version(&mut next)?);
        }
        for token in 0..self.tokens.len() {
            if self.tokens[token].kind != WorldStateIntentKind::Use {
                versions.insert(Symbol::Token(token), take_version(&mut next)?);
            }
        }

        let mut operations = Vec::new();
        for block in self.reachable.iter().copied() {
            for (slot, symbol) in entry[block].iter().copied().enumerate() {
                let Symbol::Phi(phi_block, phi_slot) = symbol else {
                    continue;
                };
                if phi_block != block || phi_slot != slot {
                    continue;
                }
                let mut incoming = BTreeMap::new();
                for predecessor in self.predecessors[block]
                    .iter()
                    .filter(|predecessor| self.reachable.contains(predecessor))
                {
                    let edge = self.edge_index(*predecessor, block)?;
                    incoming.insert(
                        block_id(*predecessor)?,
                        version_of(edge_exit[*predecessor][edge][slot], &versions)?,
                    );
                }
                operations.push(StateOp::Phi(StatePhi {
                    location: self.locations[slot].clone(),
                    version: version_of(Symbol::Phi(block, slot), &versions)?,
                    incoming,
                    includes_initial: block == entry_index,
                    site: StateSite::phi(
                        block_id(block)?,
                        u32::try_from(slot).map_err(|_| {
                            WorldStateSsaDecline::StateSiteOverflow {
                                block: self.function.blocks[block].id,
                                instruction: slot,
                            }
                        })?,
                    ),
                }));
            }
            let mut current = entry[block].clone();
            for token in &self.plans[block].base {
                self.materialise_token(*token, &mut current, &versions, &mut operations)?;
            }
            // Transition records have their definition site at execution, but
            // only edge data flow makes them visible. This preserves both the
            // Tcl sequencing and the registry's exact completion contract.
            for transition in &self.plans[block].transitions {
                for token in transition.edge_tokens.values() {
                    self.materialise_definition(*token, &versions, &mut operations)?;
                }
            }
        }
        let state = StateSsa::new(operations).map_err(WorldStateSsaDecline::InvalidStateSsa)?;
        Ok(ExecutableWorldStateSsa {
            state,
            locations: self.locations,
        })
    }

    fn join_entry(
        &self,
        block: usize,
        edge_exit: &[Vec<Vec<Symbol>>],
        slots: usize,
        includes_initial: bool,
    ) -> Result<Vec<Symbol>, WorldStateSsaDecline> {
        let mut entry = Vec::with_capacity(slots);
        for (slot, _) in self.locations.iter().enumerate() {
            let mut values = BTreeSet::new();
            if includes_initial {
                values.insert(Symbol::Initial);
            }
            for predecessor in self.predecessors[block]
                .iter()
                .filter(|predecessor| self.reachable.contains(predecessor))
            {
                let edge = self.edge_index(*predecessor, block)?;
                values.insert(edge_exit[*predecessor][edge][slot]);
            }
            let value = match values.len() {
                0 => return Err(WorldStateSsaDecline::MissingStatePredecessor { block, slot }),
                1 => *values
                    .iter()
                    .next()
                    .ok_or(WorldStateSsaDecline::MissingStatePredecessor { block, slot })?,
                _ => Symbol::Phi(block, slot),
            };
            entry.push(value);
        }
        Ok(entry)
    }

    fn apply_block_edges(&self, block: usize, mut state: Vec<Symbol>) -> Vec<Vec<Symbol>> {
        for token in &self.plans[block].base {
            self.apply_token(*token, &mut state);
        }
        self.successors[block]
            .iter()
            .copied()
            .map(|successor| {
                let mut edge = state.clone();
                for item in &self.plans[block].transitions {
                    if let Some(token) = item.edge_tokens.get(&successor) {
                        self.apply_token(*token, &mut edge);
                    }
                }
                edge
            })
            .collect()
    }

    fn edge_index(
        &self,
        predecessor: usize,
        successor: usize,
    ) -> Result<usize, WorldStateSsaDecline> {
        self.successors[predecessor]
            .iter()
            .position(|candidate| *candidate == successor)
            .ok_or(WorldStateSsaDecline::MissingCfgEdge {
                predecessor,
                successor,
            })
    }

    fn edge_mode(
        &self,
        block: usize,
        successor: usize,
    ) -> Result<EdgeCompletion, WorldStateSsaDecline> {
        let edge = self.edge_index(block, successor)?;
        self.edge_completion[block]
            .get(edge)
            .copied()
            .ok_or(WorldStateSsaDecline::MissingCfgEdge {
                predecessor: block,
                successor,
            })
    }

    fn apply_token(&self, token: usize, state: &mut [Symbol]) {
        let token_info = &self.tokens[token];
        if token_info.kind == WorldStateIntentKind::Use {
            return;
        }
        let policy = WorldRegionOverlapPolicy::conservative();
        let written = &self.locations[token_info.location];
        for (slot, read) in self.locations.iter().enumerate() {
            if policy.may_overlap(written, read) {
                state[slot] = Symbol::Token(token);
            }
        }
    }

    fn materialise_token(
        &self,
        token: usize,
        state: &mut [Symbol],
        versions: &BTreeMap<Symbol, StateVersion>,
        operations: &mut Vec<StateOp<WorldRegion>>,
    ) -> Result<(), WorldStateSsaDecline> {
        let token_info = &self.tokens[token];
        if token_info.kind == WorldStateIntentKind::Use {
            operations.push(StateOp::Use(StateUse {
                location: self.locations[token_info.location].clone(),
                reaching_version: version_of(state[token_info.location], versions)?,
                site: token_info.site.clone(),
            }));
        } else {
            self.materialise_definition(token, versions, operations)?;
            self.apply_token(token, state);
        }
        Ok(())
    }

    fn materialise_definition(
        &self,
        token: usize,
        versions: &BTreeMap<Symbol, StateVersion>,
        operations: &mut Vec<StateOp<WorldRegion>>,
    ) -> Result<(), WorldStateSsaDecline> {
        let token_info = &self.tokens[token];
        let location = self.locations[token_info.location].clone();
        let version = version_of(Symbol::Token(token), versions)?;
        match token_info.kind {
            WorldStateIntentKind::Use => {}
            WorldStateIntentKind::Def => operations.push(StateOp::Def(StateDef {
                location,
                version,
                site: token_info.site.clone(),
            })),
            WorldStateIntentKind::Clobber => operations.push(StateOp::Clobber(StateClobber {
                location,
                version,
                site: token_info.site.clone(),
            })),
        }
        Ok(())
    }
}

fn conservative_region_intents() -> Vec<(WorldStateIntentKind, WorldRegion)> {
    vec![
        (WorldStateIntentKind::Use, WorldRegion::Any),
        (WorldStateIntentKind::Clobber, WorldRegion::Any),
    ]
}

/// Whether evaluating one source word may execute nested commands.
///
/// Variable substitutions are deliberately excluded: a variable read can
/// only run code through a live variable trace, which the contents analysis
/// ([`crate::dispatch_proof`]) discharges or fails closed on explicitly.
fn word_evaluation_runs_commands(word: &crate::ir::WordExpr) -> bool {
    use crate::ir::{WordExpr, WordPart};
    match word {
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } | WordExpr::Variable { .. } => {
            false
        }
        WordExpr::CommandSubstitution { .. } | WordExpr::Opaque { .. } => true,
        WordExpr::Template { parts, .. } => parts.iter().any(|part| {
            matches!(
                part,
                WordPart::CommandSubstitution { .. } | WordPart::Opaque { .. }
            )
        }),
        WordExpr::Expand { word, .. } => word_evaluation_runs_commands(word),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Symbol {
    Initial,
    Token(usize),
    Phi(usize, usize),
}

impl Symbol {
    fn as_phi(self) -> Option<(usize, usize)> {
        match self {
            Self::Phi(block, slot) => Some((block, slot)),
            Self::Initial | Self::Token(_) => None,
        }
    }
}

fn take_version(next: &mut u32) -> Result<StateVersion, WorldStateSsaDecline> {
    let version = StateVersion::new(*next);
    *next = next
        .checked_add(1)
        .ok_or(WorldStateSsaDecline::InvalidStateSsa(
            StateSsaError::VersionExhausted,
        ))?;
    Ok(version)
}

fn version_of(
    symbol: Symbol,
    versions: &BTreeMap<Symbol, StateVersion>,
) -> Result<StateVersion, WorldStateSsaDecline> {
    match symbol {
        Symbol::Initial => Ok(StateVersion::INITIAL),
        _ => versions
            .get(&symbol)
            .copied()
            .ok_or(WorldStateSsaDecline::MissingStateVersion),
    }
}

fn block_id(index: usize) -> Result<BlockId, WorldStateSsaDecline> {
    u32::try_from(index)
        .map(BlockId)
        .map_err(|_| WorldStateSsaDecline::BlockIdOverflow { block: index })
}

pub(crate) fn successors_of(terminator: &ExecutableTerminator) -> Vec<usize> {
    match terminator {
        ExecutableTerminator::Goto(target) => vec![target.index()],
        ExecutableTerminator::Branch {
            then_target,
            else_target,
            ..
        } => {
            vec![then_target.index(), else_target.index()]
        }
        ExecutableTerminator::CompletionSwitch { cases, default, .. } => cases
            .iter()
            .map(|case| case.target.index())
            .chain(std::iter::once(default.index()))
            .collect(),
        ExecutableTerminator::ReturnCompletion(_) => Vec::new(),
    }
}

/// Completion-arm categories merged for one predecessor/successor pair.
///
/// The executable CFG identifies predecessor edges by block, while a Tcl
/// completion switch can send both `OK` and an abrupt code to the same block.
/// Retaining these bits prevents an `OK`-only transition from becoming an
/// unconditional precise definition after target deduplication.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct EdgeCompletion {
    pub(crate) ok: bool,
    pub(crate) abrupt: bool,
}

fn transition_edge_kind(
    exact_kind: WorldStateIntentKind,
    commit: StateTransitionCommit,
    edge: EdgeCompletion,
) -> Option<WorldStateIntentKind> {
    if edge.ok && !edge.abrupt {
        Some(exact_kind)
    } else if edge.abrupt
        && (commit == StateTransitionCommit::MayCommitBeforeAbruptCompletion || edge.ok)
    {
        // This is an edge-qualified representation of unchanged +
        // exact-changed state, never a sequential write in the predecessor.
        Some(WorldStateIntentKind::Clobber)
    } else {
        None
    }
}

pub(crate) fn edge_completion(
    terminator: &ExecutableTerminator,
    successor: usize,
) -> EdgeCompletion {
    let ExecutableTerminator::CompletionSwitch { cases, default, .. } = terminator else {
        return EdgeCompletion::default();
    };
    let mut completion = EdgeCompletion {
        ok: false,
        abrupt: default.index() == successor,
    };
    for case in cases {
        if case.target.index() == successor {
            if case.code == CompletionCode::Ok {
                completion.ok = true;
            } else {
                completion.abrupt = true;
            }
        }
    }
    completion
}

fn next_ordinal(
    block: ExecutableBlockId,
    instruction: usize,
    ordinal: u32,
) -> Result<u32, WorldStateSsaDecline> {
    ordinal
        .checked_add(1)
        .ok_or(WorldStateSsaDecline::StateSiteOverflow { block, instruction })
}

#[cfg(test)]
mod tests {
    use tcl_lexer::Span;
    use tcl_registry::CommandRegistry;
    use tcl_registry::model::semantic::SemanticContext;

    fn test_context() -> SemanticContext {
        SemanticContext::for_environment("tcl8.6")
    }

    use super::*;
    use crate::executable_ir::{
        CompletionCase, CompletionId, ExecutableArgvId, ExecutableBlock, ExecutableFunctionId,
        GenericInvoke, build_linear_executable_ir,
    };
    use crate::ir::{NodeId, SourceSite, WordExpr};
    use crate::lowering::lower_to_ir;
    use crate::registry_invocation::resolve_word_exprs;
    use crate::state_ssa::CfgStateSite;

    fn source() -> SourceSite {
        SourceSite::source(Span::new(0, 0))
    }

    fn literal(text: &str) -> WordExpr {
        WordExpr::Literal {
            text: text.to_owned(),
            source: source(),
        }
    }

    fn resolved_invoke(
        id: ExecutableFunctionId,
        completion: CompletionId,
        words: &[&str],
    ) -> ExecutableInstruction {
        let words: Vec<_> = words.iter().map(|word| literal(word)).collect();
        let resolution = resolve_word_exprs(
            &CommandRegistry::build_default(),
            Some(test_context()),
            &words,
        )
        .expect("test command resolves through registry");
        ExecutableInstruction::Invoke(GenericInvoke {
            completion,
            argv: ExecutableArgvId::new(id, completion.index()),
            resolution,
            original_words: words,
            node: NodeId::from_path(vec![
                u32::try_from(completion.index()).expect("test completion index fits u32"),
            ]),
            source: source(),
        })
    }

    fn switch_function(words: &[&str], same_target: bool) -> ExecutableFunction {
        let id = ExecutableFunctionId::new(71);
        let b0 = ExecutableBlockId::new(id, 0);
        let b1 = ExecutableBlockId::new(id, 1);
        let b2 = ExecutableBlockId::new(id, 2);
        let completion = CompletionId::new(id, 0);
        ExecutableFunction::new(
            id,
            b0,
            vec![
                ExecutableBlock {
                    id: b0,
                    instructions: vec![resolved_invoke(id, completion, words)],
                    terminator: Some(ExecutableTerminator::CompletionSwitch {
                        completion,
                        cases: vec![CompletionCase {
                            code: CompletionCode::Ok,
                            target: b1,
                        }],
                        default: if same_target { b1 } else { b2 },
                    }),
                },
                ExecutableBlock {
                    id: b1,
                    instructions: Vec::new(),
                    terminator: Some(ExecutableTerminator::ReturnCompletion(completion)),
                },
                ExecutableBlock {
                    id: b2,
                    instructions: Vec::new(),
                    terminator: Some(ExecutableTerminator::ReturnCompletion(completion)),
                },
            ],
        )
    }

    fn compiled_function(source: &str, id: usize) -> ExecutableFunction {
        let registry = CommandRegistry::build_default();
        let module = lower_to_ir(source, &registry);
        build_linear_executable_ir(
            &registry,
            Some(test_context()),
            ExecutableFunctionId::new(id),
            &module.top_level,
        )
        .expect("fixture source builds valid executable IR")
    }

    fn invoke_block_indices(function: &ExecutableFunction) -> Vec<usize> {
        function
            .blocks
            .iter()
            .enumerate()
            .filter_map(|(index, block)| {
                block
                    .instructions
                    .iter()
                    .any(|instruction| matches!(instruction, ExecutableInstruction::Invoke(_)))
                    .then_some(index)
            })
            .collect()
    }

    #[test]
    fn may_commit_has_exact_ok_and_joined_abrupt_versions() {
        // `rename` is a registry-declared MayCommit transition. This test
        // deliberately knows no compiler-side command semantics.
        let function = switch_function(&["rename", "old", "new"], false);
        let mut planner = Planner::new(&function).expect("small CFG fits");
        planner.plan().expect("registry completion switch is valid");
        let transition = &planner.plans[0].transitions[0];
        let edges = planner.apply_block_edges(0, vec![Symbol::Initial; planner.locations.len()]);
        let exact = *transition.edge_tokens.get(&1).expect("OK edge token");
        let joined = *transition.edge_tokens.get(&2).expect("abrupt edge token");
        let location = planner.tokens[exact].location;
        assert_eq!(edges[0][location], Symbol::Token(exact));
        assert_eq!(edges[1][location], Symbol::Token(joined));
        assert!(
            planner.plans[0]
                .base
                .iter()
                .any(|token| { planner.tokens[*token].kind == WorldStateIntentKind::Use })
        );
    }

    #[test]
    fn on_ok_transition_is_not_serialised_as_an_unconditional_world_write() {
        // `proc` declares both a legacy command-table/side-effect write and
        // an exact OnOkOnly transition. The registry coverage contract makes
        // the latter authoritative, so the definition appears only on the
        // ordinary completion edge.
        let function = switch_function(&["proc", "made", "", ""], false);
        let mut planner = Planner::new(&function).expect("small CFG fits");
        planner.plan().expect("registry completion switch is valid");
        let transition = &planner.plans[0].transitions[0];
        assert!(transition.edge_tokens.contains_key(&1));
        assert!(
            !transition.edge_tokens.contains_key(&2),
            "OnOkOnly must leave the abrupt successor unchanged"
        );
        assert!(planner.plans[0].base.iter().all(|token| {
            !matches!(
                (
                    &planner.tokens[*token].kind,
                    &planner.locations[planner.tokens[*token].location]
                ),
                (
                    WorldStateIntentKind::Def,
                    WorldRegion::Scoped {
                        kind: WorldRegionKind::CommandBindings,
                        ..
                    }
                )
            )
        }));
    }

    #[test]
    fn closed_alias_transition_has_no_unstamped_predecessor_world_barrier() {
        // `global` is closed as a direct alias operation. Unlike an
        // unstamped Invoke, its registry transition and coverage contract do
        // not fabricate an unconditional `Any` clobber before completion is
        // known.
        let function = switch_function(&["global", "shared"], false);
        let mut planner = Planner::new(&function).expect("small CFG fits");
        planner.plan().expect("registry completion switch is valid");
        assert!(planner.plans[0].base.iter().all(|token| {
            !matches!(
                (
                    &planner.tokens[*token].kind,
                    &planner.locations[planner.tokens[*token].location]
                ),
                (WorldStateIntentKind::Clobber, WorldRegion::Any)
            )
        }));
        let transition = &planner.plans[0].transitions[0];
        assert!(transition.edge_tokens.contains_key(&1));
        assert!(
            transition.edge_tokens.contains_key(&2),
            "global installs aliases one at a time and therefore preserves its MayCommit join"
        );
    }

    #[test]
    fn mixed_ok_and_default_target_uses_join_not_precise_definition() {
        let function = switch_function(&["rename", "old", "new"], true);
        let mut planner = Planner::new(&function).expect("small CFG fits");
        planner.plan().expect("registry completion switch is valid");
        let transition = &planner.plans[0].transitions[0];
        let edge = planner.apply_block_edges(0, vec![Symbol::Initial; planner.locations.len()]);
        let joined = *transition
            .edge_tokens
            .get(&1)
            .expect("mixed edge join token");
        let location = planner.tokens[joined].location;
        assert_eq!(edge[0][location], Symbol::Token(joined));
    }

    #[test]
    fn unreachable_transition_does_not_change_the_planned_state_graph() {
        let mut function = switch_function(&["rename", "old", "new"], false);
        let unreachable = ExecutableBlockId::new(function.id, 3);
        let completion = CompletionId::new(function.id, 1);
        function.blocks.push(ExecutableBlock {
            id: unreachable,
            instructions: vec![resolved_invoke(
                function.id,
                completion,
                &["proc", "dead", "", ""],
            )],
            terminator: Some(ExecutableTerminator::ReturnCompletion(completion)),
        });
        let mut planner = Planner::new(&function).expect("small CFG fits");
        planner
            .plan()
            .expect("dead transition imposes no completion-edge requirement");
        assert!(planner.locations.iter().all(|location| !matches!(
            location,
            WorldRegion::Scoped {
                kind: WorldRegionKind::CommandBindings,
                subject: WorldSubjectScope::Named(name),
                ..
            } if name == "dead"
        )));
    }

    #[test]
    fn entry_backedge_merges_the_implicit_initial_world_state() {
        let mut function = compiled_function("rename old new\n", 72);
        let invoke = invoke_block_indices(&function)[0];
        let entry = function.entry;
        let ExecutableInstruction::Invoke(generic) = &function.blocks[invoke].instructions[0]
        else {
            panic!("fixture invocation block contains an invoke");
        };
        let completion = generic.completion;
        let abrupt = ExecutableBlockId::new(function.id, function.blocks.len());
        function.blocks[invoke].terminator = Some(ExecutableTerminator::CompletionSwitch {
            completion,
            cases: vec![CompletionCase {
                code: CompletionCode::Ok,
                target: entry,
            }],
            default: abrupt,
        });
        function.blocks.push(ExecutableBlock {
            id: abrupt,
            instructions: Vec::new(),
            terminator: Some(ExecutableTerminator::ReturnCompletion(completion)),
        });
        let invoke_block = block_id(invoke).expect("fixture block index fits shared BlockId");

        let world = build_executable_world_state_ssa(&function)
            .expect("entry backedge has a representable state merge");
        assert!(world.state.operations().iter().any(|operation| matches!(
            operation,
            StateOp::Phi(phi)
                if phi.includes_initial()
                    && matches!(&phi.site, StateSite::Cfg(CfgStateSite {
                        block: BlockId(0),
                        position: CfgStatePosition::Phi { .. },
                    }))
                    && phi.incoming.contains_key(&invoke_block)
        )));
    }

    #[test]
    fn namespace_and_trace_widening_cover_every_partition() {
        let facts = [
            StateTransitionFact {
                transition: StateTransition::Widen(tcl_registry::StateTransitionWidening {
                    domains: vec![StateTransitionDomain::Namespaces],
                    subject: TransitionSubject::Literal("dynamic".to_owned()),
                }),
                commit: StateTransitionCommit::OnOkOnly,
            },
            StateTransitionFact {
                transition: StateTransition::Widen(tcl_registry::StateTransitionWidening {
                    domains: vec![StateTransitionDomain::CommandTraces],
                    subject: TransitionSubject::Literal("dynamic".to_owned()),
                }),
                commit: StateTransitionCommit::OnOkOnly,
            },
            StateTransitionFact {
                transition: StateTransition::Widen(tcl_registry::StateTransitionWidening {
                    domains: vec![StateTransitionDomain::ExecutionTraces],
                    subject: TransitionSubject::Literal("dynamic".to_owned()),
                }),
                commit: StateTransitionCommit::OnOkOnly,
            },
        ];
        let locations: Vec<_> = project_transition_facts(&facts)
            .into_iter()
            .map(|intent| intent.location)
            .collect();
        assert!(locations.iter().any(|location| matches!(
            location,
            WorldRegion::Scoped {
                kind: WorldRegionKind::NamespaceLookup,
                ..
            }
        )));
        assert!(locations.iter().any(|location| matches!(
            location,
            WorldRegion::Scoped {
                kind: WorldRegionKind::NamespaceUnknown,
                ..
            }
        )));
        assert!(locations.iter().any(|location| matches!(
            location,
            WorldRegion::Scoped {
                kind: WorldRegionKind::CommandTraces,
                ..
            }
        )));
        assert!(locations.iter().any(|location| matches!(
            location,
            WorldRegion::Scoped {
                kind: WorldRegionKind::ExecutionTraces,
                ..
            }
        )));
    }

    #[test]
    fn trace_transition_projection_keeps_command_and_execution_tables_separate() {
        let facts = [
            StateTransitionFact {
                transition: StateTransition::Trace(tcl_registry::TraceTransition::Add {
                    target: tcl_registry::TraceTarget::Command(TransitionSubject::Literal(
                        "worker".to_owned(),
                    )),
                    operations: tcl_registry::TraceOperationSet::Known(vec![
                        tcl_registry::TraceOperation::Rename,
                    ]),
                    prefix: TransitionSubject::Literal("callback".to_owned()),
                }),
                commit: StateTransitionCommit::OnOkOnly,
            },
            StateTransitionFact {
                transition: StateTransition::Trace(tcl_registry::TraceTransition::Add {
                    target: tcl_registry::TraceTarget::Execution(TransitionSubject::Literal(
                        "worker".to_owned(),
                    )),
                    operations: tcl_registry::TraceOperationSet::Known(vec![
                        tcl_registry::TraceOperation::Enter,
                    ]),
                    prefix: TransitionSubject::Literal("callback".to_owned()),
                }),
                commit: StateTransitionCommit::OnOkOnly,
            },
        ];
        let locations: Vec<_> = project_transition_facts(&facts)
            .into_iter()
            .map(|intent| intent.location)
            .collect();
        assert!(locations.iter().any(|location| matches!(
            location,
            WorldRegion::Scoped {
                kind: WorldRegionKind::CommandTraces,
                subject: WorldSubjectScope::Named(name),
                ..
            } if name == "worker"
        )));
        assert!(locations.iter().any(|location| matches!(
            location,
            WorldRegion::Scoped {
                kind: WorldRegionKind::ExecutionTraces,
                subject: WorldSubjectScope::Named(name),
                ..
            } if name == "worker"
        )));
    }

    #[test]
    fn variable_trace_removal_projects_placeholder_cell_and_trace_table() {
        let facts = [StateTransitionFact {
            transition: StateTransition::Trace(tcl_registry::TraceTransition::Remove {
                target: tcl_registry::TraceTarget::Variable(TransitionSubject::Literal(
                    "item".to_owned(),
                )),
                operations: tcl_registry::TraceOperationSet::Known(vec![
                    tcl_registry::TraceOperation::Write,
                ]),
                prefix: TransitionSubject::Literal("callback".to_owned()),
            }),
            commit: StateTransitionCommit::OnOkOnly,
        }];
        let intents = project_transition_facts(&facts);

        for kind in [
            WorldRegionKind::VariableStore,
            WorldRegionKind::VariableTraces,
        ] {
            assert!(intents.iter().any(|intent| {
                intent.kind == WorldStateIntentKind::Use
                    && matches!(
                        &intent.location,
                        WorldRegion::Scoped {
                            kind: actual,
                            subject: WorldSubjectScope::Named(name),
                            ..
                        } if *actual == kind && name == "item"
                    )
            }));
            assert!(intents.iter().any(|intent| {
                intent.kind == WorldStateIntentKind::Def
                    && matches!(
                        &intent.location,
                        WorldRegion::Scoped {
                            kind: actual,
                            subject: WorldSubjectScope::Named(name),
                            ..
                        } if *actual == kind && name == "item"
                    )
            }));
        }
    }

    #[test]
    fn namespace_transition_projection_preserves_regions_and_commit_policy() {
        let facts = [
            StateTransitionFact {
                transition: StateTransition::Namespace(NamespaceTransition::Ensure {
                    namespace: NamespaceTransitionTarget::Named(TransitionSubject::Literal(
                        "::a::b".to_owned(),
                    )),
                }),
                commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
            },
            StateTransitionFact {
                transition: StateTransition::Namespace(NamespaceTransition::Delete {
                    namespace: NamespaceTransitionTarget::Named(TransitionSubject::Literal(
                        "::gone".to_owned(),
                    )),
                }),
                commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
            },
        ];
        let intents = project_transition_facts(&facts);

        assert!(intents.iter().any(|intent| matches!(
            &intent.location,
            WorldRegion::NamespaceLineage {
                interpreter: WorldInterpreterScope::Current,
                namespace: WorldNamespaceScope::Named(namespace),
            } if namespace == "::a::b"
        ) && intent.commit
            == StateTransitionCommit::MayCommitBeforeAbruptCompletion));
        assert!(intents.iter().any(|intent| matches!(
            &intent.location,
            WorldRegion::NamespaceSubtree {
                interpreter: WorldInterpreterScope::Current,
                namespace: WorldNamespaceScope::Named(namespace),
            } if namespace == "::gone"
        ) && intent.commit
            == StateTransitionCommit::MayCommitBeforeAbruptCompletion));
    }

    #[test]
    fn rename_target_namespace_transition_projects_its_created_lineage() {
        // The registry emits this companion fact for `rename old
        // ::created::target`; Tcl creates the target namespace if it is
        // missing.  The world-state projection must retain that fact rather
        // than treating the move as only a command-table update.
        let facts = [StateTransitionFact {
            transition: StateTransition::Namespace(NamespaceTransition::Ensure {
                namespace: NamespaceTransitionTarget::Named(TransitionSubject::Literal(
                    "::created".to_owned(),
                )),
            }),
            commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
        }];

        assert!(
            project_transition_facts(&facts)
                .iter()
                .any(|intent| matches!(
                    &intent.location,
                    WorldRegion::NamespaceLineage {
                        interpreter: WorldInterpreterScope::Current,
                        namespace: WorldNamespaceScope::Named(namespace),
                    } if namespace == "::created"
                ) && intent.commit
                    == StateTransitionCommit::MayCommitBeforeAbruptCompletion)
        );
    }

    #[test]
    fn interpreter_transitions_project_lifecycle_policy_and_reverse_alias_safety() {
        let facts = [
            StateTransitionFact {
                transition: StateTransition::Interpreter(InterpreterTransition::Create {
                    interpreter: Some(TransitionSubject::Literal("child".to_owned())),
                    safety: tcl_registry::ChildInterpreterSafety::Safe,
                }),
                commit: StateTransitionCommit::OnOkOnly,
            },
            StateTransitionFact {
                transition: StateTransition::Interpreter(InterpreterTransition::Delete {
                    interpreter: TransitionSubject::Literal("child".to_owned()),
                }),
                commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
            },
        ];
        let intents = project_transition_facts(&facts);

        assert!(intents.iter().any(|intent| matches!(
            &intent.location,
            WorldRegion::InterpreterWildcard {
                interpreter: WorldInterpreterScope::Named(path),
            } if path == "child"
        )));
        assert!(intents.iter().any(|intent| matches!(
            &intent.location,
            WorldRegion::Scoped {
                kind: WorldRegionKind::InterpreterTopology,
                interpreter: WorldInterpreterScope::Named(path),
                ..
            } if path == "child"
        )));
        assert!(intents.iter().any(|intent| matches!(
            (&intent.kind, &intent.location),
            (WorldStateIntentKind::Clobber, WorldRegion::Scoped {
                kind: WorldRegionKind::CommandBindings,
                interpreter: WorldInterpreterScope::Any,
                namespace: WorldNamespaceScope::Any,
                subject: WorldSubjectScope::Wildcard,
            }) if intent.commit == StateTransitionCommit::MayCommitBeforeAbruptCompletion
        )));
    }

    #[test]
    fn object_lifecycle_projection_keeps_dispatch_namespace_and_private_variables() {
        let facts = [StateTransitionFact {
            transition: StateTransition::ObjectDispatch(ObjectDispatchTransition::Create {
                target: ObjectDispatchTarget::Named(TransitionSubject::Literal(
                    "::object".to_owned(),
                )),
                private_namespace: ObjectPrivateNamespace::Fresh,
                kind: tcl_registry::ObjectDispatchKind::Object,
            }),
            commit: StateTransitionCommit::OnOkOnly,
        }];
        let intents = project_transition_facts(&facts);

        assert!(intents.iter().any(|intent| matches!(
            (&intent.kind, &intent.location),
            (
                WorldStateIntentKind::Def,
                WorldRegion::Scoped {
                    kind: WorldRegionKind::ObjectDispatch,
                    subject: WorldSubjectScope::Named(name),
                    ..
                }
            ) if name == "::object"
        )));
        assert!(intents.iter().any(|intent| matches!(
            (&intent.kind, &intent.location),
            (
                WorldStateIntentKind::Def,
                WorldRegion::Scoped {
                    kind: WorldRegionKind::NamespaceLookup,
                    namespace: WorldNamespaceScope::Any,
                    ..
                }
            )
        )));
        assert!(intents.iter().any(|intent| matches!(
            (&intent.kind, &intent.location),
            (
                WorldStateIntentKind::Clobber,
                WorldRegion::Scoped {
                    kind: WorldRegionKind::VariableStore,
                    namespace: WorldNamespaceScope::Any,
                    ..
                }
            )
        )));
        assert!(
            intents
                .iter()
                .all(|intent| intent.commit == StateTransitionCommit::OnOkOnly)
        );
    }
}
