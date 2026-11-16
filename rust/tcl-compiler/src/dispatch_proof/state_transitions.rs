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

//! The registry `StateTransition` facts applied to the ledgers: command bindings, interpreters, aliases, namespaces, traces and objects.

use super::world_tracks::{TrackVersion, WorldTrack};
use super::{
    SubjectPattern, TraceKey, WorldContents, interpreter_may_be_current, namespace_alias_target,
    normalise_name,
};
use tcl_registry::{
    CommandBindingTransition, InterpreterTransition, NamespaceTransition,
    NamespaceTransitionTarget, ObjectDispatchTarget, ObjectDispatchTransition, StateTransition,
    StateTransitionDomain, TraceOperationSet, TraceTarget, TraceTransition, VariableAliasTarget,
};

pub(super) fn apply_transition_fact(
    state: &mut WorldContents,
    transition: &StateTransition,
    site: TrackVersion,
) {
    match transition {
        StateTransition::CommandBinding(transition) => {
            apply_command_binding(state, transition, site);
        }
        StateTransition::Interpreter(transition) => {
            apply_interpreter_transition(state, transition, site);
        }
        StateTransition::VariableCellAlias(alias) => {
            apply_variable_alias(state, alias, site);
        }
        StateTransition::Namespace(transition) => {
            apply_namespace_transition(state, transition, site);
        }
        StateTransition::Trace(transition) => apply_trace_transition(state, transition, site),
        StateTransition::ObjectDispatch(transition) => {
            apply_object_transition(state, transition, site);
        }
        StateTransition::Package(transition) => {
            use tcl_registry::model::binding::PackageTransition;
            let writes = match transition {
                PackageTransition::Provide { version, .. } => version.is_some(),
                PackageTransition::Ifneeded {
                    script_provided, ..
                } => *script_provided,
                PackageTransition::UnknownHandler { handler } => handler.is_some(),
                PackageTransition::Prefer { mode } => mode.is_some(),
                PackageTransition::Require { .. }
                | PackageTransition::Forget { .. }
                | PackageTransition::SourceLoad { .. }
                | PackageTransition::IndexedAutoload { .. }
                | PackageTransition::DiscoveryDependencyChanged { .. } => true,
            };
            if writes {
                state.bump(WorldTrack::PackageState, site);
            }
        }
        StateTransition::Widen(widening) => {
            for domain in &widening.domains {
                apply_domain_widening(state, *domain, site);
            }
        }
    }
}

fn apply_command_binding(
    state: &mut WorldContents,
    transition: &CommandBindingTransition,
    site: TrackVersion,
) {
    match transition {
        CommandBindingTransition::Define { name, .. } => state.bindings.record_subject(name),
        CommandBindingTransition::Move { from, to } => {
            state.bindings.record_subject(from);
            state.bindings.record_subject(to);
        }
        CommandBindingTransition::Delete { interpreter, name } => match interpreter {
            None => state.bindings.record_subject(name),
            Some(interpreter) if interpreter_may_be_current(interpreter) => {
                state.bindings.record_subject(name);
            }
            // A deletion inside a named child interpreter cannot unbind a
            // current-interpreter command.
            Some(_) => return,
        },
        CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            ..
        } => {
            if !interpreter_may_be_current(source_interpreter) {
                return;
            }
            state.bindings.record_subject(alias);
        }
        CommandBindingTransition::Unknown { .. } => state.bindings.widen(),
    }
    state.bump(WorldTrack::CommandBindings, site);
}

fn apply_interpreter_transition(
    state: &mut WorldContents,
    transition: &InterpreterTransition,
    site: TrackVersion,
) {
    match transition {
        InterpreterTransition::Create { interpreter, .. } => {
            state.bump(WorldTrack::InterpreterTopology, site);
            // The child-path command appears in the current interpreter; a
            // generated child name is fresh and cannot collide with an
            // existing resolution.
            if let Some(subject) = interpreter {
                state.bindings.record_subject(subject);
                state.bump(WorldTrack::CommandBindings, site);
            }
        }
        InterpreterTransition::Delete { .. } => {
            // Any surviving interpreter can retain an alias whose target was
            // the deleted child; reverse alias edges are not tracked.
            state.bindings.widen();
            state.bump(WorldTrack::CommandBindings, site);
            state.bump(WorldTrack::InterpreterTopology, site);
        }
        InterpreterTransition::MarkTrusted { interpreter }
        | InterpreterTransition::SetRecursionLimit { interpreter, .. }
        | InterpreterTransition::SetBackgroundError { interpreter, .. } => {
            if interpreter_may_be_current(interpreter) {
                state.interpreter_policy_stable = false;
            }
            state.bump(WorldTrack::InterpreterPolicy, site);
        }
        InterpreterTransition::Hide {
            interpreter,
            visible,
            ..
        }
        | InterpreterTransition::Expose {
            interpreter,
            visible,
            ..
        } => {
            if interpreter_may_be_current(interpreter) {
                state.interpreter_policy_stable = false;
                state.bindings.record_subject(visible);
                state.bump(WorldTrack::CommandBindings, site);
            }
            state.bump(WorldTrack::InterpreterPolicy, site);
        }
    }
}

fn apply_variable_alias(
    state: &mut WorldContents,
    alias: &tcl_registry::VariableCellAliasTransition,
    site: TrackVersion,
) {
    let target = match &alias.target {
        VariableAliasTarget::Global { variable }
        | VariableAliasTarget::CurrentNamespace { variable } => variable
            .literal()
            .map(|name| normalise_name(name).into_owned()),
        VariableAliasTarget::CallerSelectedFrame { .. } => None,
        VariableAliasTarget::Namespace {
            namespace,
            variable,
        } => match (namespace.literal(), variable.literal()) {
            (Some(namespace), Some(variable)) => Some(namespace_alias_target(namespace, variable)),
            _ => None,
        },
    };
    match alias.local.literal() {
        Some(local) => state
            .aliases
            .record(normalise_name(local).into_owned(), target),
        None => state.aliases.widen(),
    }
    state.bump(WorldTrack::VariableStore, site);
}

fn apply_namespace_transition(
    state: &mut WorldContents,
    transition: &NamespaceTransition,
    site: TrackVersion,
) {
    let prefix = |target: &NamespaceTransitionTarget| -> Option<SubjectPattern> {
        match target {
            // Analysed units start in the global namespace, so the current
            // namespace is the global prefix.
            NamespaceTransitionTarget::Current => {
                Some(SubjectPattern::NamespacePrefix(String::new()))
            }
            NamespaceTransitionTarget::Named(subject)
                if subject
                    .literal()
                    .is_some_and(tcl_syntax::naming::is_absolute) =>
            {
                Some(SubjectPattern::NamespacePrefix(
                    normalise_name(subject.literal().expect("qualified namespace value"))
                        .into_owned(),
                ))
            }
            NamespaceTransitionTarget::Named(_) => None,
        }
    };
    match transition {
        NamespaceTransition::Ensure { .. } | NamespaceTransition::Export { .. } => {
            // Creating an empty namespace binds no command; export lists
            // gate future imports.  Neither changes an existing resolution.
            state.bump(WorldTrack::NamespaceLookup, site);
        }
        NamespaceTransition::Delete { namespace } => {
            state.namespace_lookup_stable = false;
            state.namespace_unknown_stable = false;
            if let Some(pattern) = prefix(namespace) {
                state.bindings.record(pattern.clone());
                state.object_dispatch.record(pattern);
            } else {
                state.bindings.widen();
                state.object_dispatch.widen();
            }
            state.bump(WorldTrack::NamespaceLookup, site);
            state.bump(WorldTrack::NamespaceUnknown, site);
            state.bump(WorldTrack::CommandBindings, site);
            state.bump(WorldTrack::ObjectDispatch, site);
            state.bump(WorldTrack::VariableStore, site);
        }
        NamespaceTransition::Import { namespace, .. }
        | NamespaceTransition::Forget { namespace, .. }
        | NamespaceTransition::Ensemble { namespace, .. } => {
            state.namespace_lookup_stable = false;
            match prefix(namespace) {
                Some(pattern) => state.bindings.record(pattern),
                None => state.bindings.widen(),
            }
            state.bump(WorldTrack::NamespaceLookup, site);
            state.bump(WorldTrack::CommandBindings, site);
        }
        NamespaceTransition::SetPath { .. } => {
            state.namespace_lookup_stable = false;
            state.bump(WorldTrack::NamespaceLookup, site);
        }
        NamespaceTransition::SetUnknown { .. } => {
            state.namespace_unknown_stable = false;
            state.bump(WorldTrack::NamespaceUnknown, site);
        }
    }
}

fn apply_trace_transition(
    state: &mut WorldContents,
    transition: &TraceTransition,
    site: TrackVersion,
) {
    let (adding, target, operations, prefix) = match transition {
        TraceTransition::Add {
            target,
            operations,
            prefix,
        } => (true, target, operations, prefix),
        TraceTransition::Remove {
            target,
            operations,
            prefix,
        } => (false, target, operations, prefix),
    };
    let (track, subject) = match target {
        TraceTarget::Variable(subject) => (WorldTrack::VariableTraces, subject),
        TraceTarget::Command(subject) => (WorldTrack::CommandTraces, subject),
        TraceTarget::Execution(subject) => (WorldTrack::ExecutionTraces, subject),
    };
    if matches!(target, TraceTarget::Variable(_)) {
        // Variable trace registration vivifies its cell; removal of the last
        // trace releases the placeholder.
        state.bump(WorldTrack::VariableStore, site);
    }
    state.bump(track, site);
    let ledger = match track {
        WorldTrack::VariableTraces => &mut state.variable_traces,
        WorldTrack::CommandTraces => &mut state.command_traces,
        _ => &mut state.execution_traces,
    };
    let Some(name) = subject.literal() else {
        if adding {
            ledger.widen();
        } else {
            // A dynamic removal target removes at most one registration
            // somewhere: presence upper bounds are unaffected.
            for (_, cell) in &mut ledger.cells {
                cell.remove_any();
            }
        }
        return;
    };
    let name = normalise_name(name);
    let known_operations = match operations {
        TraceOperationSet::Known(operations) => Some(operations.clone()),
        TraceOperationSet::Unknown(_) => None,
    };
    let literal_prefix = prefix.literal().map(str::to_owned);
    let Some(cell) = ledger.cell_mut(&name) else {
        return;
    };
    match (adding, known_operations) {
        (true, Some(operations)) => {
            cell.add(TraceKey {
                operations,
                prefix: literal_prefix,
            });
        }
        (true, None) => cell.unknown = true,
        (false, Some(operations)) => {
            cell.remove(&TraceKey {
                operations,
                prefix: literal_prefix,
            });
        }
        (false, None) => cell.remove_any(),
    }
}

fn apply_object_transition(
    state: &mut WorldContents,
    transition: &ObjectDispatchTransition,
    site: TrackVersion,
) {
    match transition {
        ObjectDispatchTransition::Configure { target, .. } => {
            state.object_dispatch.record_subject(target);
        }
        ObjectDispatchTransition::Destroy { target } => {
            state.object_dispatch.record_subject(target);
            // Destroying an object also deletes its public command.
            state.bindings.record_subject(target);
            state.bump(WorldTrack::CommandBindings, site);
        }
        ObjectDispatchTransition::Create { target, .. }
        | ObjectDispatchTransition::Copy { target, .. } => match target {
            ObjectDispatchTarget::Named(subject) => state.object_dispatch.record_subject(subject),
            // Generated object names are fresh and cannot collide with an
            // existing resolution.
            ObjectDispatchTarget::Fresh => {}
        },
    }
    state.bump(WorldTrack::ObjectDispatch, site);
}

fn apply_domain_widening(
    state: &mut WorldContents,
    domain: StateTransitionDomain,
    site: TrackVersion,
) {
    match domain {
        StateTransitionDomain::CommandBindings => {
            state.bindings.widen();
            state.bump(WorldTrack::CommandBindings, site);
        }
        StateTransitionDomain::VariableCells => {
            if state.variable_traces.completely_absent() && !state.aliases.unknown {
                state.aliases.widen();
                state.bump(WorldTrack::VariableStore, site);
            } else {
                // A write to an unknown cell may fire arbitrary handlers.
                state.widen_all(site);
            }
        }
        StateTransitionDomain::Namespaces | StateTransitionDomain::CommandResolution => {
            state.namespace_lookup_stable = false;
            state.namespace_unknown_stable = false;
            state.bump(WorldTrack::NamespaceLookup, site);
            state.bump(WorldTrack::NamespaceUnknown, site);
        }
        StateTransitionDomain::InterpreterTopology => {
            state.bump(WorldTrack::InterpreterTopology, site);
        }
        StateTransitionDomain::InterpreterPolicy => {
            state.interpreter_policy_stable = false;
            state.bump(WorldTrack::InterpreterPolicy, site);
        }
        StateTransitionDomain::Interpreters => state.widen_all(site),
        StateTransitionDomain::CommandTraces => {
            state.command_traces.widen();
            state.bump(WorldTrack::CommandTraces, site);
        }
        StateTransitionDomain::ExecutionTraces => {
            state.execution_traces.widen();
            state.bump(WorldTrack::ExecutionTraces, site);
        }
        StateTransitionDomain::VariableTraces => {
            state.variable_traces.widen();
            state.bump(WorldTrack::VariableTraces, site);
        }
        StateTransitionDomain::ObjectDispatch => {
            state.object_dispatch.widen();
            state.bump(WorldTrack::ObjectDispatch, site);
        }
        StateTransitionDomain::Packages => state.bump(WorldTrack::PackageState, site),
    }
}
