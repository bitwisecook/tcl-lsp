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

//! Projecting registry transition facts into version-free state intents: packages, objects, command bindings, interpreters, aliases, namespaces and traces, and the regions they name.

use super::{TransitionStateIntent, WorldStateSsaDecline};
use crate::effect_ssa::{WorldStateIntentKind, project_effect_footprint};
use crate::executable_ir::{ExecutableBlockId, InvocationResolution};
use crate::state_ssa::StateSite;
use crate::state_ssa::adapters::{
    WorldInterpreterScope, WorldNamespaceScope, WorldRegion, WorldRegionKind, WorldSubjectScope,
};
use tcl_registry::{
    CommandBindingTransition, InterpreterTransition, NamespaceTransition,
    NamespaceTransitionTarget, ObjectDispatchTarget, ObjectDispatchTransition,
    ObjectPrivateNamespace, StateTransition, StateTransitionCommit, StateTransitionDomain,
    StateTransitionFact, StateTransitionKnowledge, TraceTarget, TraceTransition, TransitionSubject,
    VariableAliasTarget,
};

pub(super) type InvocationIntents = (
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

pub(super) fn project_invocation_intents(
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
        StateTransition::Package(transition) => {
            append_package(intents, fact.commit, transition);
        }
        StateTransition::Widen(widening) => append_widening(intents, fact.commit, widening),
    }
}

fn append_package(
    intents: &mut Vec<TransitionStateIntent>,
    commit: StateTransitionCommit,
    transition: &tcl_registry::model::binding::PackageTransition,
) {
    use tcl_registry::model::binding::PackageTransition;
    let region = |subject: Option<&TransitionSubject>| {
        subject.and_then(TransitionSubject::literal).map_or_else(
            || unknown_current_region(WorldRegionKind::PackageState),
            |name| {
                WorldRegion::exact(
                    WorldRegionKind::PackageState,
                    WorldInterpreterScope::Current,
                    WorldNamespaceScope::Any,
                    name,
                )
            },
        )
    };
    let (subjects, writes): (Vec<Option<&TransitionSubject>>, bool) = match transition {
        PackageTransition::Provide { package, version } => (vec![Some(package)], version.is_some()),
        PackageTransition::Require { package, .. } => (vec![Some(package)], true),
        PackageTransition::Ifneeded {
            package,
            script_provided,
            ..
        } => (vec![Some(package)], *script_provided),
        PackageTransition::Forget { packages } => (packages.iter().map(Some).collect(), true),
        PackageTransition::UnknownHandler { handler } => (vec![None], handler.is_some()),
        PackageTransition::Prefer { mode } => (vec![None], mode.is_some()),
        PackageTransition::SourceLoad { .. }
        | PackageTransition::IndexedAutoload { .. }
        | PackageTransition::DiscoveryDependencyChanged { .. } => (vec![None], true),
    };
    for subject in subjects {
        let location = region(subject);
        push_intent(intents, commit, WorldStateIntentKind::Use, location.clone());
        if writes {
            push_intent(intents, commit, WorldStateIntentKind::Clobber, location);
        }
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
    // and dispatch-chain reasoning. This state partition aliases both layers
    // deliberately: proving them disjoint would require the class/object
    // ownership graph, which executable IR does not carry.
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
    // distinct in TclOO, and dispatch state carries no reverse mapping, so
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
    // derived from its public command spelling, and dispatch state carries no
    // reverse mapping, so destruction invalidates both private partitions.
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
            // Baked leading arguments change what the alias *runs*, never
            // which command regions it reads or writes.
            ..
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
    // deleted child, and the state topology carries no reverse alias edges.
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
        NamespaceTransition::Import { namespace, .. }
        | NamespaceTransition::Forget {
            namespace,
            patterns: _,
        }
        | NamespaceTransition::Ensemble { namespace, .. } => {
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
        NamespaceTransitionTarget::Named(subject)
            if subject.literal().is_some_and(crate::naming::is_absolute) =>
        {
            WorldNamespaceScope::named(subject.literal().expect("qualified namespace value"))
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
        // A generated object is fresh, but the invocation facts carry no
        // CFG-site identity for it.  The lifecycle transition is retained
        // while conservatively overlapping every object.
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
        TransitionSubject::Literal(value) | TransitionSubject::LocatedLiteral { value, .. } => {
            WorldSubjectScope::named(value)
        }
        TransitionSubject::LocatedNativeBytes { .. } | TransitionSubject::Unknown { .. } => {
            WorldSubjectScope::Wildcard
        }
    }
}

fn subject_interpreter(subject: &TransitionSubject) -> WorldInterpreterScope {
    match subject {
        TransitionSubject::Literal(value) | TransitionSubject::LocatedLiteral { value, .. } => {
            WorldInterpreterScope::named(value)
        }
        TransitionSubject::LocatedNativeBytes { .. } | TransitionSubject::Unknown { .. } => {
            WorldInterpreterScope::Any
        }
    }
}

fn optional_subject_interpreter(subject: Option<&TransitionSubject>) -> WorldInterpreterScope {
    subject.map_or(WorldInterpreterScope::Any, subject_interpreter)
}

fn subject_namespace(subject: &TransitionSubject) -> WorldNamespaceScope {
    match subject {
        TransitionSubject::Literal(value) | TransitionSubject::LocatedLiteral { value, .. } => {
            WorldNamespaceScope::named(value)
        }
        TransitionSubject::LocatedNativeBytes { .. } | TransitionSubject::Unknown { .. } => {
            WorldNamespaceScope::Any
        }
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
        StateTransitionDomain::Namespaces | StateTransitionDomain::CommandResolution => vec![
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
        StateTransitionDomain::Packages => vec![region(WorldRegionKind::PackageState)],
    }
}
