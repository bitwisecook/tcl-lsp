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

//! `trace` — monitor variable accesses, command usages and executions.
use crate::prelude::*;
use crate::world_effect::{
    EffectAccess, EffectAccessMode, EffectFootprint, InterpreterScope, NamespaceScope, SubjectScope,
};
use tcl_cmd_core::trace::{TraceKind, parse_legacy_variable_ops, parse_ops, resolve_type};
use tcl_dialect::model::Family;
use tcl_dialect::model::SpecSurface;
use tcl_dialect::surface;

const VARIABLE_TRACE_OPERATIONS: &[&str] = &["array", "read", "unset", "write"];
const COMMAND_TRACE_OPERATIONS: &[&str] = &["delete", "rename"];
const EXECUTION_TRACE_OPERATIONS: &[&str] = &["enter", "leave", "enterstep", "leavestep"];
const LEGACY_VARIABLE_TRACE_OPERATIONS: &[&str] = &["r", "w", "u", "a"];
const EXECUTION_TRACE_CALLBACK_ARITIES: AppendedAritySet =
    AppendedAritySet::from_sorted_unique(&[2, 4]);

fn operations_for(kind: TraceKind) -> &'static [&'static str] {
    match kind {
        TraceKind::Variable => VARIABLE_TRACE_OPERATIONS,
        TraceKind::Command => COMMAND_TRACE_OPERATIONS,
        TraceKind::Execution => EXECUTION_TRACE_OPERATIONS,
    }
}

fn operation_subject(kind: TraceKind) -> &'static str {
    match kind {
        TraceKind::Variable => "variable trace operation list",
        TraceKind::Command => "command trace operation list",
        TraceKind::Execution => "execution trace operation list",
    }
}

/// Validate the modern `trace add|remove type name opList prefix` relationship.
///
/// The type word accepts a unique prefix, but operation-list members are exact.
/// This callback deliberately abstains when arity, type, or literalness is not
/// proven; the generic arity/closed-value/parser diagnostics own those cases.
fn validate_modern_trace_operations(
    arguments: InvocationArguments<'_>,
) -> LiteralArgumentValidation {
    if arguments.exact_argv_len() != Some(5) {
        return LiteralArgumentValidation::Abstain(LiteralValidationDecline::IncompleteInvocation);
    }
    let Some(type_word) = arguments.literal_at(1) else {
        return LiteralArgumentValidation::Abstain(LiteralValidationDecline::NonLiteralArgument);
    };
    let Ok(kind) = resolve_type(type_word) else {
        return LiteralArgumentValidation::Abstain(LiteralValidationDecline::InvalidDiscriminator);
    };
    let Some(op_list) = arguments.literal_at(3) else {
        return LiteralArgumentValidation::Abstain(LiteralValidationDecline::NonLiteralArgument);
    };
    let Ok(elements) = tcl_syntax::list::split_list(op_list) else {
        return LiteralArgumentValidation::Abstain(LiteralValidationDecline::MalformedLiteral);
    };
    let allowed = operations_for(kind);
    if elements.is_empty() {
        return LiteralArgumentValidation::Invalid(LiteralArgumentIssue {
            argument_index: 3,
            subject: operation_subject(kind),
            reason: LiteralArgumentIssueReason::Empty,
            allowed_values: allowed,
            replacement_value: None,
        });
    }
    let invalid: Vec<String> = elements
        .iter()
        .filter(|element| !allowed.contains(&element.as_ref()))
        .map(ToString::to_string)
        .collect();
    if invalid.is_empty() {
        return LiteralArgumentValidation::Valid;
    }
    let valid: Vec<&str> = elements
        .iter()
        .map(AsRef::as_ref)
        .filter(|element| allowed.contains(element))
        .collect();
    let replacement_value = (!valid.is_empty()).then(|| tcl_syntax::list::join_list(valid));
    LiteralArgumentValidation::Invalid(LiteralArgumentIssue {
        argument_index: 3,
        subject: operation_subject(kind),
        reason: LiteralArgumentIssueReason::InvalidMembers(invalid),
        allowed_values: allowed,
        replacement_value,
    })
}

/// Validate Tcl 8.x's deprecated concatenated `rwua` operation string.
fn validate_legacy_trace_operations(
    arguments: InvocationArguments<'_>,
) -> LiteralArgumentValidation {
    if arguments.exact_argv_len() != Some(4) {
        return LiteralArgumentValidation::Abstain(LiteralValidationDecline::IncompleteInvocation);
    }
    let Some(operations) = arguments.literal_at(2) else {
        return LiteralArgumentValidation::Abstain(LiteralValidationDecline::NonLiteralArgument);
    };
    if operations.is_empty() {
        return LiteralArgumentValidation::Invalid(LiteralArgumentIssue {
            argument_index: 2,
            subject: "legacy variable trace operation string",
            reason: LiteralArgumentIssueReason::Empty,
            allowed_values: LEGACY_VARIABLE_TRACE_OPERATIONS,
            replacement_value: None,
        });
    }
    let invalid: Vec<String> = operations
        .chars()
        .filter(|operation| !matches!(operation, 'r' | 'w' | 'u' | 'a'))
        .map(|operation| operation.to_string())
        .collect();
    if invalid.is_empty() {
        LiteralArgumentValidation::Valid
    } else {
        LiteralArgumentValidation::Invalid(LiteralArgumentIssue {
            argument_index: 2,
            subject: "legacy variable trace operation string",
            reason: LiteralArgumentIssueReason::InvalidMembers(invalid),
            allowed_values: LEGACY_VARIABLE_TRACE_OPERATIONS,
            // This is not a Tcl list. Avoid presenting the modern
            // list-member removal action for the deprecated encoding.
            replacement_value: None,
        })
    }
}

// Command-level fallback for call sites where the actual subcommand can't
// be resolved statically (`dialect_side_effect_hints` in
// `tcl-compiler/src/side_effects.rs` prefers a resolved subcommand's own
// `side_effects` over this one, so this only fires as the conservative
// default). Declares the union of what any subcommand can do: `info`,
// `vinfo`, `remove`, and `vdelete` read the existing trace table, while
// `add`, `remove`, `variable`, and `vdelete` write it.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "trace option ?arg arg ...?",
    ..FormSpec::DEFAULT
}];

const TRACE_ADD_TRANSITION_DOMAINS: &[StateTransitionDomain] = &[
    StateTransitionDomain::VariableCells,
    StateTransitionDomain::CommandTraces,
    StateTransitionDomain::ExecutionTraces,
    StateTransitionDomain::VariableTraces,
];

const TRACE_REMOVE_TRANSITION_DOMAINS: &[StateTransitionDomain] = &[
    StateTransitionDomain::VariableCells,
    StateTransitionDomain::CommandTraces,
    StateTransitionDomain::ExecutionTraces,
    StateTransitionDomain::VariableTraces,
];

const TRACE_INFO_RESULT_DOMAINS: &[WorldStateDomain] = &[
    WorldStateDomain::VariableTraces,
    WorldStateDomain::CommandTraces,
    WorldStateDomain::ExecutionTraces,
];

const TRACE_EFFECT_COVERAGE: &[TransitionEffectCoverage] = &[
    TransitionEffectCoverage {
        // `trace` still has its long-standing `InterpState` side effect while
        // consumers migrate to trace-specific domains. The resolved
        // transition owns the write portion for successful add/remove calls;
        // its residual legacy read remains conservative migration debt.
        source: WorldEffectWriteSource::LegacySideEffect(SideEffectTarget::InterpState),
        domains: &[WorldStateDomain::InterpreterPolicy],
    },
    TransitionEffectCoverage {
        // Trace registration tables are cumulative, so the explicit
        // descriptor retains its read dependency. The completion-qualified
        // transition is authoritative only for the write component.
        source: WorldEffectWriteSource::DeclaredWorldEffect,
        domains: &[
            WorldStateDomain::VariableStore,
            WorldStateDomain::VariableTraces,
            WorldStateDomain::CommandTraces,
            WorldStateDomain::ExecutionTraces,
        ],
    },
];

const LEGACY_TRACE_EFFECT_COVERAGE: &[TransitionEffectCoverage] = &[
    TransitionEffectCoverage {
        source: WorldEffectWriteSource::LegacySideEffect(SideEffectTarget::InterpState),
        domains: &[WorldStateDomain::InterpreterPolicy],
    },
    TransitionEffectCoverage {
        source: WorldEffectWriteSource::DeclaredWorldEffect,
        domains: &[
            WorldStateDomain::VariableStore,
            WorldStateDomain::VariableTraces,
        ],
    },
];

const TRACE_ADD_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(trace_add_state_transitions),
    argument_shape: StateTransitionArgumentShape::Positional,
    dynamic_widening: &[StateTransitionWideningRule {
        operands: StateTransitionOperandLayout::Indices(&[1]),
        domains: TRACE_ADD_TRANSITION_DOMAINS,
    }],
    effect_coverage: TRACE_EFFECT_COVERAGE,
    // `trace add` and `trace remove` make no synchronous callback.  Their
    // registration changes only become observable after Tcl returns `OK`.
    commit: StateTransitionCommit::OnOkOnly,
};

const TRACE_REMOVE_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(trace_remove_state_transitions),
    argument_shape: StateTransitionArgumentShape::Positional,
    dynamic_widening: &[StateTransitionWideningRule {
        operands: StateTransitionOperandLayout::Indices(&[1]),
        domains: TRACE_REMOVE_TRANSITION_DOMAINS,
    }],
    effect_coverage: TRACE_EFFECT_COVERAGE,
    commit: StateTransitionCommit::OnOkOnly,
};

const LEGACY_TRACE_ADD_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(legacy_trace_add_state_transitions),
    argument_shape: StateTransitionArgumentShape::Positional,
    dynamic_widening: &[],
    effect_coverage: LEGACY_TRACE_EFFECT_COVERAGE,
    commit: StateTransitionCommit::OnOkOnly,
};

const LEGACY_TRACE_REMOVE_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(legacy_trace_remove_state_transitions),
    argument_shape: StateTransitionArgumentShape::Positional,
    dynamic_widening: &[],
    effect_coverage: LEGACY_TRACE_EFFECT_COVERAGE,
    commit: StateTransitionCommit::OnOkOnly,
};

const TRACE_ADD_DYNAMIC_EFFECT_ACCESSES: &[StaticEffectAccess] = &[
    StaticEffectAccess::new(
        WorldStateDomain::VariableStore,
        EffectAccessMode::ReadWrite,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    ),
    StaticEffectAccess::new(
        WorldStateDomain::VariableTraces,
        EffectAccessMode::ReadWrite,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    ),
    StaticEffectAccess::new(
        WorldStateDomain::CommandTraces,
        EffectAccessMode::ReadWrite,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    ),
    StaticEffectAccess::new(
        WorldStateDomain::ExecutionTraces,
        EffectAccessMode::ReadWrite,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    ),
    StaticEffectAccess::new(
        WorldStateDomain::CommandBindings,
        EffectAccessMode::Read,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    ),
    StaticEffectAccess::new(
        WorldStateDomain::NamespaceLookup,
        EffectAccessMode::Read,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    ),
];

const TRACE_REMOVE_DYNAMIC_EFFECT_ACCESSES: &[StaticEffectAccess] = &[
    StaticEffectAccess::new(
        WorldStateDomain::VariableTraces,
        EffectAccessMode::ReadWrite,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    ),
    StaticEffectAccess::new(
        WorldStateDomain::CommandTraces,
        EffectAccessMode::ReadWrite,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    ),
    StaticEffectAccess::new(
        WorldStateDomain::ExecutionTraces,
        EffectAccessMode::ReadWrite,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    ),
    StaticEffectAccess::new(
        WorldStateDomain::CommandBindings,
        EffectAccessMode::Read,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    ),
    StaticEffectAccess::new(
        WorldStateDomain::NamespaceLookup,
        EffectAccessMode::Read,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    ),
];

const TRACE_INFO_DYNAMIC_EFFECT_ACCESSES: &[StaticEffectAccess] = &[
    StaticEffectAccess::new(
        WorldStateDomain::VariableTraces,
        EffectAccessMode::Read,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    ),
    StaticEffectAccess::new(
        WorldStateDomain::CommandTraces,
        EffectAccessMode::Read,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    ),
    StaticEffectAccess::new(
        WorldStateDomain::ExecutionTraces,
        EffectAccessMode::Read,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    ),
    StaticEffectAccess::new(
        WorldStateDomain::CommandBindings,
        EffectAccessMode::Read,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    ),
    StaticEffectAccess::new(
        WorldStateDomain::NamespaceLookup,
        EffectAccessMode::Read,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    ),
];

const LEGACY_TRACE_DYNAMIC_EFFECT_ACCESSES: &[StaticEffectAccess] = &[
    StaticEffectAccess::new(
        WorldStateDomain::VariableStore,
        EffectAccessMode::ReadWrite,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    ),
    StaticEffectAccess::new(
        WorldStateDomain::VariableTraces,
        EffectAccessMode::ReadWrite,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    ),
];

const LEGACY_TRACE_INFO_DYNAMIC_EFFECT_ACCESSES: &[StaticEffectAccess] =
    &[StaticEffectAccess::new(
        WorldStateDomain::VariableTraces,
        EffectAccessMode::Read,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    )];

const TRACE_ADD_EFFECTS: WorldEffectDescriptor = WorldEffectDescriptor {
    composition: WorldEffectComposition::Extend,
    static_footprint: StaticEffectFootprint::EMPTY,
    resolver: Some(trace_add_world_effects),
    dynamic_fallback: WorldEffectDynamicFallback::Declared(StaticEffectFootprint {
        accesses: TRACE_ADD_DYNAMIC_EFFECT_ACCESSES,
        callback: CallbackEffect::NONE,
    }),
};

const TRACE_REMOVE_EFFECTS: WorldEffectDescriptor = WorldEffectDescriptor {
    composition: WorldEffectComposition::Extend,
    static_footprint: StaticEffectFootprint::EMPTY,
    resolver: Some(trace_remove_world_effects),
    dynamic_fallback: WorldEffectDynamicFallback::Declared(StaticEffectFootprint {
        accesses: TRACE_REMOVE_DYNAMIC_EFFECT_ACCESSES,
        callback: CallbackEffect::NONE,
    }),
};

const TRACE_INFO_EFFECTS: WorldEffectDescriptor = WorldEffectDescriptor {
    composition: WorldEffectComposition::Extend,
    static_footprint: StaticEffectFootprint::EMPTY,
    resolver: Some(trace_info_world_effects),
    dynamic_fallback: WorldEffectDynamicFallback::Declared(StaticEffectFootprint {
        accesses: TRACE_INFO_DYNAMIC_EFFECT_ACCESSES,
        callback: CallbackEffect::NONE,
    }),
};

const LEGACY_TRACE_ADD_EFFECTS: WorldEffectDescriptor = WorldEffectDescriptor {
    composition: WorldEffectComposition::Extend,
    static_footprint: StaticEffectFootprint::EMPTY,
    resolver: Some(legacy_trace_add_world_effects),
    dynamic_fallback: WorldEffectDynamicFallback::Declared(StaticEffectFootprint {
        accesses: LEGACY_TRACE_DYNAMIC_EFFECT_ACCESSES,
        callback: CallbackEffect::NONE,
    }),
};

const LEGACY_TRACE_REMOVE_EFFECTS: WorldEffectDescriptor = WorldEffectDescriptor {
    composition: WorldEffectComposition::Extend,
    static_footprint: StaticEffectFootprint::EMPTY,
    resolver: Some(legacy_trace_remove_world_effects),
    dynamic_fallback: WorldEffectDynamicFallback::Declared(StaticEffectFootprint {
        accesses: LEGACY_TRACE_DYNAMIC_EFFECT_ACCESSES,
        callback: CallbackEffect::NONE,
    }),
};

const LEGACY_TRACE_INFO_EFFECTS: WorldEffectDescriptor = WorldEffectDescriptor {
    composition: WorldEffectComposition::Extend,
    static_footprint: StaticEffectFootprint::EMPTY,
    resolver: Some(legacy_trace_info_world_effects),
    dynamic_fallback: WorldEffectDynamicFallback::Declared(StaticEffectFootprint {
        accesses: LEGACY_TRACE_INFO_DYNAMIC_EFFECT_ACCESSES,
        callback: CallbackEffect::NONE,
    }),
};

fn trace_target(kind: TraceKind, target: TransitionSubject) -> TraceTarget {
    match kind {
        TraceKind::Variable => TraceTarget::Variable(target),
        TraceKind::Command => TraceTarget::Command(target),
        TraceKind::Execution => TraceTarget::Execution(target),
    }
}

fn trace_operation(operation: &str) -> Option<TraceOperation> {
    match operation {
        "array" => Some(TraceOperation::Array),
        "read" => Some(TraceOperation::Read),
        "write" => Some(TraceOperation::Write),
        "unset" => Some(TraceOperation::Unset),
        "rename" => Some(TraceOperation::Rename),
        "delete" => Some(TraceOperation::Delete),
        "enter" => Some(TraceOperation::Enter),
        "leave" => Some(TraceOperation::Leave),
        "enterstep" => Some(TraceOperation::EnterStep),
        "leavestep" => Some(TraceOperation::LeaveStep),
        _ => None,
    }
}

fn trace_operations(kind: TraceKind, op_list: TransitionSubject) -> Option<TraceOperationSet> {
    match op_list {
        TransitionSubject::Literal(op_list) => parse_ops(op_list.as_bytes(), kind)
            .ok()?
            .into_iter()
            .map(trace_operation)
            .collect::<Option<Vec<_>>>()
            .map(TraceOperationSet::Known),
        unknown @ TransitionSubject::Unknown { .. } => Some(TraceOperationSet::Unknown(unknown)),
    }
}

fn trace_state_transition(
    arguments: InvocationArguments<'_>,
    action: fn(TraceTarget, TraceOperationSet, TransitionSubject) -> TraceTransition,
) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    if arguments.exact_argv_len() != Some(5) {
        return transitions;
    }
    let Some(kind) = arguments.literal_at(1).and_then(resolve_trace_type) else {
        return transitions;
    };
    let (Some(target), Some(op_list), Some(prefix)) = (
        TransitionSubject::from_argument(arguments, 2),
        TransitionSubject::from_argument(arguments, 3),
        TransitionSubject::from_argument(arguments, 4),
    ) else {
        return transitions;
    };
    let Some(operations) = trace_operations(kind, op_list) else {
        return transitions;
    };
    transitions.push(StateTransition::Trace(action(
        trace_target(kind, target),
        operations,
        prefix,
    )));
    transitions
}

fn trace_add_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    trace_state_transition(arguments, |target, operations, prefix| {
        TraceTransition::Add {
            target,
            operations,
            prefix,
        }
    })
}

fn trace_remove_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    trace_state_transition(arguments, |target, operations, prefix| {
        TraceTransition::Remove {
            target,
            operations,
            prefix,
        }
    })
}

fn legacy_trace_operations(op_string: TransitionSubject) -> Option<TraceOperationSet> {
    match op_string {
        TransitionSubject::Literal(op_string) => {
            // The legacy parser already yields the same canonical set the
            // modern one does, so both spellings model identically.
            parse_legacy_variable_ops(op_string.as_bytes())
                .ok()?
                .into_iter()
                .map(trace_operation)
                .collect::<Option<Vec<_>>>()
                .map(TraceOperationSet::Known)
        }
        unknown @ TransitionSubject::Unknown { .. } => Some(TraceOperationSet::Unknown(unknown)),
    }
}

fn legacy_trace_state_transition(
    arguments: InvocationArguments<'_>,
    action: fn(TraceTarget, TraceOperationSet, TransitionSubject) -> TraceTransition,
) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    if arguments.exact_argv_len() != Some(4) {
        return transitions;
    }
    let (Some(target), Some(op_string), Some(prefix)) = (
        TransitionSubject::from_argument(arguments, 1),
        TransitionSubject::from_argument(arguments, 2),
        TransitionSubject::from_argument(arguments, 3),
    ) else {
        return transitions;
    };
    let Some(operations) = legacy_trace_operations(op_string) else {
        return transitions;
    };
    transitions.push(StateTransition::Trace(action(
        TraceTarget::Variable(target),
        operations,
        prefix,
    )));
    transitions
}

fn legacy_trace_add_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    legacy_trace_state_transition(arguments, |target, operations, prefix| {
        TraceTransition::Add {
            target,
            operations,
            prefix,
        }
    })
}

fn legacy_trace_remove_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    legacy_trace_state_transition(arguments, |target, operations, prefix| {
        TraceTransition::Remove {
            target,
            operations,
            prefix,
        }
    })
}

fn trace_effect_access(
    footprint: &mut EffectFootprint,
    domain: WorldStateDomain,
    mode: EffectAccessMode,
    target: &str,
) {
    footprint.add_access(EffectAccess::new(
        domain,
        mode,
        InterpreterScope::Current,
        NamespaceScope::Current,
        SubjectScope::named(target),
    ));
}

fn trace_world_effects(
    arguments: InvocationArguments<'_>,
    trace_mode: EffectAccessMode,
    vivifies_variable: bool,
) -> EffectFootprint {
    let mut footprint = EffectFootprint::default();
    let Some(kind) = arguments.literal_at(1).and_then(resolve_trace_type) else {
        return footprint;
    };
    let Some(target) = arguments.literal_at(2) else {
        return footprint;
    };
    match kind {
        TraceKind::Variable => {
            if vivifies_variable {
                trace_effect_access(
                    &mut footprint,
                    WorldStateDomain::VariableStore,
                    EffectAccessMode::ReadWrite,
                    target,
                );
            }
            trace_effect_access(
                &mut footprint,
                WorldStateDomain::VariableTraces,
                trace_mode,
                target,
            );
        }
        TraceKind::Command | TraceKind::Execution => {
            trace_effect_access(
                &mut footprint,
                WorldStateDomain::CommandBindings,
                EffectAccessMode::Read,
                target,
            );
            trace_effect_access(
                &mut footprint,
                WorldStateDomain::NamespaceLookup,
                EffectAccessMode::Read,
                target,
            );
            if let Some(domain) = match kind {
                TraceKind::Command => Some(WorldStateDomain::CommandTraces),
                TraceKind::Execution => Some(WorldStateDomain::ExecutionTraces),
                TraceKind::Variable => None,
            } {
                trace_effect_access(&mut footprint, domain, trace_mode, target);
            }
        }
    }
    footprint
}

fn trace_add_world_effects(arguments: InvocationArguments<'_>) -> EffectFootprint {
    trace_world_effects(arguments, EffectAccessMode::ReadWrite, true)
}

fn trace_remove_world_effects(arguments: InvocationArguments<'_>) -> EffectFootprint {
    // Removing the last trace from an otherwise undefined variable destroys
    // the placeholder cell that `trace add variable` created. This is visible
    // through `namespace which -variable`, so removal touches both domains.
    trace_world_effects(arguments, EffectAccessMode::ReadWrite, true)
}

fn trace_info_world_effects(arguments: InvocationArguments<'_>) -> EffectFootprint {
    trace_world_effects(arguments, EffectAccessMode::Read, false)
}

fn legacy_trace_world_effects(
    arguments: InvocationArguments<'_>,
    trace_mode: EffectAccessMode,
) -> EffectFootprint {
    let mut footprint = EffectFootprint::default();
    let Some(target) = arguments.literal_at(1) else {
        return footprint;
    };
    trace_effect_access(
        &mut footprint,
        WorldStateDomain::VariableStore,
        EffectAccessMode::ReadWrite,
        target,
    );
    trace_effect_access(
        &mut footprint,
        WorldStateDomain::VariableTraces,
        trace_mode,
        target,
    );
    footprint
}

fn legacy_trace_add_world_effects(arguments: InvocationArguments<'_>) -> EffectFootprint {
    legacy_trace_world_effects(arguments, EffectAccessMode::ReadWrite)
}

fn legacy_trace_remove_world_effects(arguments: InvocationArguments<'_>) -> EffectFootprint {
    legacy_trace_world_effects(arguments, EffectAccessMode::ReadWrite)
}

fn legacy_trace_info_world_effects(arguments: InvocationArguments<'_>) -> EffectFootprint {
    let mut footprint = EffectFootprint::default();
    let Some(target) = arguments.literal_at(1) else {
        return footprint;
    };
    trace_effect_access(
        &mut footprint,
        WorldStateDomain::VariableTraces,
        EffectAccessMode::Read,
        target,
    );
    footprint
}

/// Resolve a `trace add|remove` type word (`variable`/`command`/
/// `execution`) the way C Tcl 9.0's `Tcl_GetIndexFromObj` does: a
/// unique, non-empty prefix is accepted, so `trace add v x read h` /
/// `trace add var x read h` install the same variable trace as the
/// full spelling (checked against tclsh 8.6.14).
fn resolve_trace_type(word: &str) -> Option<TraceKind> {
    resolve_type(word).ok()
}

/// Arg-role resolver for `trace add`.
///
/// `trace add variable name ops commandPrefix` writes to `name` —
/// the trace handler can rewrite the variable at runtime, so SSA
/// must see `name` as a definition site.
///
/// The resolver only fires for the `variable` form (accepting any
/// unique-prefix abbreviation of it, e.g. `var`/`v`) so
/// `trace add execution` and `trace add command` (which take a
/// command name, not a variable) don't appear as SSA defs.
fn trace_add_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() < 2 {
        return Vec::new();
    }
    match args.first().and_then(|w| resolve_trace_type(w)) {
        // The traced variable can be rewritten by the handler, so SSA must see
        // `name` as a definition site.
        Some(TraceKind::Variable) => vec![(1, ArgRole::VarWrite)],
        // `command` / `execution` trace a command by name — a reference
        // navigation follows to the named command (not invoked here; the
        // handler prefix that follows is a separate `CommandPrefix`).
        Some(TraceKind::Command | TraceKind::Execution) => vec![(1, ArgRole::CommandName)],
        _ => Vec::new(),
    }
}

/// Same arg-role pattern for `trace remove variable` — keeps
/// registry consistency with `trace add variable` so consumers can
/// query both spellings via the same `ArgRole::VarWrite` lookup.
fn trace_remove_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() < 2 {
        return Vec::new();
    }
    match args.first().and_then(|w| resolve_trace_type(w)) {
        Some(TraceKind::Variable) => vec![(1, ArgRole::VarWrite)],
        Some(TraceKind::Command | TraceKind::Execution) => vec![(1, ArgRole::CommandName)],
        _ => Vec::new(),
    }
}

/// The invoked arity of a `trace add/remove <type> name ops cmdPrefix`
/// callback (C Tcl 9.0): a `variable` trace fires `cmdPrefix name1 name2 op`
/// (3), a `command` trace fires `cmdPrefix oldName newName op` (3), an
/// `execution` trace fires exactly 2 args for `enter`/`enterstep` and exactly
/// 4 for `leave`/`leavestep`; a mixed literal list therefore requires the
/// callback to accept the finite union `{2, 4}`. A `remove` only references
/// the handler (it is matched, not invoked), so it carries `Unknown` —
/// recorded as a reference, not arity-checked.
///
/// The operation word is a Tcl list. Dynamic, malformed, empty, or invalid
/// lists carry `Unknown`: the registration cannot be proved to install any
/// particular callback contract, and the literal validator/parser owns its
/// own diagnostics.
fn trace_type_command_prefix(
    args: CommandPrefixArguments<'_>,
    installing: bool,
) -> Vec<(u8, AppendedArity)> {
    // args after the subcommand word: `type name ops cmdPrefix` (index 3).
    if args.len() <= 3 {
        return Vec::new();
    }
    let arity = if installing {
        let Some(kind) = args.literal_at(0).and_then(resolve_trace_type) else {
            return vec![(3, AppendedArity::Unknown)];
        };
        let Some(operation_list) = args.literal_at(2) else {
            return vec![(3, AppendedArity::Unknown)];
        };
        let Ok(operations) = tcl_syntax::list::split_list(operation_list) else {
            return vec![(3, AppendedArity::Unknown)];
        };
        let allowed = operations_for(kind);
        if operations.is_empty()
            || operations
                .iter()
                .any(|operation| !allowed.contains(&operation.as_ref()))
        {
            return vec![(3, AppendedArity::Unknown)];
        }
        match kind {
            TraceKind::Variable | TraceKind::Command => AppendedArity::Exactly(3),
            TraceKind::Execution => {
                let has_two = operations
                    .iter()
                    .any(|operation| matches!(operation.as_ref(), "enter" | "enterstep"));
                let has_four = operations
                    .iter()
                    .any(|operation| matches!(operation.as_ref(), "leave" | "leavestep"));
                match (has_two, has_four) {
                    (true, false) => AppendedArity::Exactly(2),
                    (false, true) => AppendedArity::Exactly(4),
                    (true, true) => AppendedArity::OneOf(EXECUTION_TRACE_CALLBACK_ARITIES),
                    (false, false) => AppendedArity::Unknown,
                }
            }
        }
    } else {
        AppendedArity::Unknown
    };
    vec![(3, arity)]
}

fn trace_add_command_prefixes(args: CommandPrefixArguments<'_>) -> Vec<(u8, AppendedArity)> {
    trace_type_command_prefix(args, true)
}

fn trace_add_script_timing(args: &[&str]) -> Vec<(u8, ScriptTiming)> {
    (args.len() > 3)
        .then_some((3, ScriptTiming::Deferred))
        .into_iter()
        .collect()
}

fn trace_remove_command_prefixes(args: CommandPrefixArguments<'_>) -> Vec<(u8, AppendedArity)> {
    trace_type_command_prefix(args, false)
}

fn trace_remove_script_timing(args: &[&str]) -> Vec<(u8, ScriptTiming)> {
    (args.len() > 3)
        .then_some((3, ScriptTiming::ReferenceOnly))
        .into_iter()
        .collect()
}

/// Deprecated `trace variable name ops command` / `trace vdelete …` — the
/// command prefix is the 3rd word (index 2).  `variable` installs a
/// variable trace (`command name1 name2 op` → 3 args); `vdelete` only
/// references the handler (`Unknown`).
fn trace_legacy_command_prefix(
    args: CommandPrefixArguments<'_>,
    installing: bool,
) -> Vec<(u8, AppendedArity)> {
    if args.len() <= 2 {
        return Vec::new();
    }
    let arity = if installing {
        AppendedArity::Exactly(3)
    } else {
        AppendedArity::Unknown
    };
    vec![(2, arity)]
}

fn trace_variable_command_prefixes(args: CommandPrefixArguments<'_>) -> Vec<(u8, AppendedArity)> {
    trace_legacy_command_prefix(args, true)
}

fn trace_variable_script_timing(args: &[&str]) -> Vec<(u8, ScriptTiming)> {
    (args.len() > 2)
        .then_some((2, ScriptTiming::Deferred))
        .into_iter()
        .collect()
}

fn trace_vdelete_command_prefixes(args: CommandPrefixArguments<'_>) -> Vec<(u8, AppendedArity)> {
    trace_legacy_command_prefix(args, false)
}

fn trace_vdelete_script_timing(args: &[&str]) -> Vec<(u8, ScriptTiming)> {
    (args.len() > 2)
        .then_some((2, ScriptTiming::ReferenceOnly))
        .into_iter()
        .collect()
}

/// The `type` word taken by `trace add|remove|info` (relative index 0,
/// right after the subcommand word) — `command`, `execution`, or
/// `variable`, or any prefix of one of those that is unique among the
/// three (`Tcl_GetIndexFromObj`-style abbreviation; see
/// [`resolve_trace_type`], which implements the identical matching rule
/// for the arg-role/command-prefix resolvers above). Spelled identically
/// in the `trace add type name ops ?args?` synopsis line of every Tcl
/// release from 8.4 through 9.1 — the trace.n manpage is unchanged here
/// across the whole range. `type` is always exactly one word, never a
/// list, so (unlike [`TRACE_OPS_VALUES`] below) this set is exhaustive:
/// closed via `closed_value_args` with `arg_values_accept_prefix: true`.
const TRACE_TYPE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "command",
        detail: "Trace renames and deletions of a command.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "execution",
        detail: "Trace invocation of a command, and optionally every command nested inside it.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "variable",
        detail: "Trace reads, writes, unsets, and array operations on a variable.",
        ..ArgValue::DEFAULT
    },
];

/// The `ops`/`opList` value taken by `trace add|remove` (relative index
/// 2) — a Tcl LIST of one or more of the ten words below, the legal set
/// depending on which `type` (position 0) the call names: `command` type
/// takes `rename`/`delete`; `execution` type takes
/// `enter`/`leave`/`enterstep`/`leavestep`; `variable` type takes
/// `array`/`read`/`write`/`unset`. All ten words are spelled identically
/// in every Tcl release from 8.4 through 9.1. Because this position is a
/// *list* (`trace add variable x {read write} cb` is one argument
/// carrying two ops words), it is completion/hover data only — never
/// `closed_value_args` (see the pitfall noted on `open`'s `ACCESS_VALUES`
/// for the same reasoning: a value here can legitimately combine several
/// of these words, which whole-word `closed_value_args` matching cannot
/// express).
const TRACE_OPS_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "array",
        detail: "variable type: fires when the variable is accessed via the array command, while it is not (yet) a scalar.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "read",
        detail: "variable type: fires whenever the variable is read.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "write",
        detail: "variable type: fires whenever the variable is written.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "unset",
        detail: "variable type: fires whenever the variable is unset, explicitly or via procedure return.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "rename",
        detail: "command type: fires when the traced command is renamed to a non-empty name.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "delete",
        detail: "command type: fires when the traced command is deleted (including a rename to the empty string).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "enter",
        detail: "execution type: fires just before the traced command runs.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "leave",
        detail: "execution type: fires just after the traced command runs.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "enterstep",
        detail: "execution type: fires just before every command executed inside the traced procedure.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "leavestep",
        detail: "execution type: fires just after every command executed inside the traced procedure.",
        ..ArgValue::DEFAULT
    },
];

/// The `ops` value taken by the deprecated `trace variable`/`trace
/// vdelete` (relative index 1) — NOT a Tcl list like the modern
/// [`TRACE_OPS_VALUES`], but per the trace.n manpage (unchanged across
/// 8.4-8.6, the only versions where this form exists) "a string
/// concatenation of the operations" with no separator, e.g. `rwu` for
/// read+write+unset: `array`/`read`/`write`/`unset` are abbreviated
/// `a`/`r`/`w`/`u` respectively. Combinable the same way `ops` itself is,
/// so likewise never `closed_value_args`.
const TRACE_LEGACY_OPS_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "a",
        detail: "array: fires when the variable is accessed via the array command.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "r",
        detail: "read: fires whenever the variable is read.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "w",
        detail: "write: fires whenever the variable is written.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "u",
        detail: "unset: fires whenever the variable is unset.",
        ..ArgValue::DEFAULT
    },
];

/// Arg-role resolver for the deprecated `trace variable name ops
/// command` / `trace vdelete name ops command` legacy forms — the
/// variable name is the word immediately after the subcommand
/// (relative index 0), mirroring [`trace_add_arg_roles`] for the
/// modern `trace add variable` spelling so SSA sees the same
/// definition-site behaviour regardless of which form the source
/// uses.
fn trace_legacy_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.is_empty() {
        return Vec::new();
    }
    vec![(0, ArgRole::VarWrite)]
}

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        // `trace add command|execution NAME …` targets a command by its
        // spelled name, so command identities are observable data here too.
        traits: Traits::TARGETS_VARIABLE_BY_NAME
            .union(Traits::ESTABLISHES_VARIABLE_TRACE)
            .union(Traits::REFLECTS_COMMAND_NAMES)
            .union(Traits::DEFERS_BODY),
        arity: Arity::exact(4),
        detail: "Arrange for commandPrefix to be invoked, with operation-specific arguments appended, whenever the variable/command/execution named by name undergoes one of the operations in ops. For type command or execution, name must already exist or this throws an error; for type variable, a nonexistent name is instead silently created without a value (visible to a namespace which query but not to info exists, until something writes it). Present, with this exact 4-argument shape, in every Tcl release from 8.4 through 9.1.",
        synopsis: "trace add type name ops commandPrefix",
        return_type: Some(TclType::String),
        mutator: true,
        arg_role_resolver: Some(trace_add_arg_roles),
        arg_role_resolver_roles: &[ArgRole::VarWrite, ArgRole::CommandName],
        command_prefix_resolver: Some(trace_add_command_prefixes),
        script_timing_resolver: Some(trace_add_script_timing),
        arg_values: &[(0, TRACE_TYPE_VALUES), (2, TRACE_OPS_VALUES)],
        closed_value_args: &[0],
        arg_values_accept_prefix: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        world_effects: Some(TRACE_ADD_EFFECTS),
        state_transitions: Some(TRACE_ADD_TRANSITIONS),
        literal_argument_validator: Some(validate_modern_trace_operations),
        // measurements §5 (`docs/design/bigip-irule-parser-measurements.md`,
        // BIG-IP 21.1.0.1): TMM's `trace` is the 8.3-era form ONLY — `trace
        // add variable …` fails with `wrong # args` — so the modern ensemble
        // subcommands carry `ALL_TCL` (every real Tcl core, no iRules row) and
        // never intersect the bare iRules mask. This is an arity/form gate on
        // the embedded fork, not a command removal: `trace` itself stays
        // present in iRules.
        surface: Some(SpecSurface::ALL_TCL),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        // `trace info command|execution NAME` reads traces off a command
        // named as data.
        traits: Traits::TARGETS_VARIABLE_BY_NAME.union(Traits::REFLECTS_COMMAND_NAMES),
        arity: Arity::exact(2),
        detail: "Return a list of the traces currently set on the command or variable name, one element per trace, each itself a two-element {opList commandPrefix} list, or an empty list if none are set. For type command or execution, a nonexistent name throws an error; for type variable, a nonexistent name likewise just yields an empty list.",
        synopsis: "trace info type name",
        pure: true,
        return_type: Some(TclType::List),
        result_stability: Some(ResultStability::ReadsVersionedWorld(
            TRACE_INFO_RESULT_DOMAINS,
        )),
        arg_values: &[(0, TRACE_TYPE_VALUES)],
        closed_value_args: &[0],
        arg_values_accept_prefix: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        world_effects: Some(TRACE_INFO_EFFECTS),
        state_transitions: Some(StateTransitionDescriptor::EMPTY),
        // measurements §5: 8.3-form-only on TMM — see `add` above.
        surface: Some(SpecSurface::ALL_TCL),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "remove",
        // `trace remove command|execution NAME …` targets a command by its
        // spelled name.
        traits: Traits::TARGETS_VARIABLE_BY_NAME
            .union(Traits::ESTABLISHES_VARIABLE_TRACE)
            .union(Traits::REFLECTS_COMMAND_NAMES),
        arity: Arity::exact(4),
        detail: "Remove a previously added trace matching type, name, ops, and commandPrefix exactly. For a variable name that does not exist, or a variable/command/execution trace that does not match, this is silently a no-op; for a nonexistent command or execution name it instead throws an error.",
        synopsis: "trace remove type name opList commandPrefix",
        return_type: Some(TclType::String),
        mutator: true,
        arg_role_resolver: Some(trace_remove_arg_roles),
        arg_role_resolver_roles: &[ArgRole::VarWrite, ArgRole::CommandName],
        command_prefix_resolver: Some(trace_remove_command_prefixes),
        script_timing_resolver: Some(trace_remove_script_timing),
        arg_values: &[(0, TRACE_TYPE_VALUES), (2, TRACE_OPS_VALUES)],
        closed_value_args: &[0],
        arg_values_accept_prefix: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        world_effects: Some(TRACE_REMOVE_EFFECTS),
        state_transitions: Some(TRACE_REMOVE_TRANSITIONS),
        literal_argument_validator: Some(validate_modern_trace_operations),
        // measurements §5: 8.3-form-only on TMM — see `add` above.
        surface: Some(SpecSurface::ALL_TCL),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "variable",
        traits: Traits::TARGETS_VARIABLE_BY_NAME
            .union(Traits::ESTABLISHES_VARIABLE_TRACE)
            .union(Traits::DEFERS_BODY),
        arity: Arity::exact(3),
        detail: "Arrange for command to be executed whenever variable name is accessed, using the legacy single-letter ops encoding (a/r/w/u, concatenated with no separator, e.g. rwu). Equivalent to trace add variable name ops command. Deprecated throughout 8.4-8.6; removed in Tcl 9.0 (absent from the 9.0/9.1 manpages' synopsis and body alike).",
        synopsis: "trace variable name ops command",
        return_type: Some(TclType::String),
        mutator: true,
        arg_role_resolver: Some(trace_legacy_arg_roles),
        arg_role_resolver_roles: &[ArgRole::VarWrite],
        command_prefix_resolver: Some(trace_variable_command_prefixes),
        script_timing_resolver: Some(trace_variable_script_timing),
        arg_values: &[(1, TRACE_LEGACY_OPS_VALUES)],
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        world_effects: Some(LEGACY_TRACE_ADD_EFFECTS),
        state_transitions: Some(LEGACY_TRACE_ADD_TRANSITIONS),
        literal_argument_validator: Some(validate_legacy_trace_operations),
        // Deprecated legacy form; removed in Tcl 9.0 (8.4-8.6 only) — the
        // 9.0/9.1 trace.n manpages drop both the SYNOPSIS line and the
        // entire "For backwards compatibility..." paragraph describing it.
        // `dialects` is the membership mask; `lifecycle` states the same two
        // boundaries as ordered releases on the Tcl core axis, so a file
        // targeting 8.x is told the form is deprecated (W144) and a
        // `package require Tcl` range spanning 9.0 is told it does not hold
        // across the whole range. The `IRULES` bit is measured
        // (measurements §5, BIG-IP 21.1.0.1): TMM's `trace` accepts the
        // 8.3-era forms ONLY — this one works where `trace add` is
        // `wrong # args` — an arity/form gate, not a removal.
        surface: Some(surface![
            SpecSurface::core_in(Family::Tcl, &[("8.4", Some("8.7"))]),
            SpecSurface::core(Family::F5Irules)
        ]),
        lifecycle: Lifecycle::deprecated_in("8.4").retired_from("9.0"),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vdelete",
        traits: Traits::TARGETS_VARIABLE_BY_NAME.union(Traits::ESTABLISHES_VARIABLE_TRACE),
        arity: Arity::exact(3),
        detail: "Delete a variable trace added with trace variable or trace add variable, using the legacy single-letter ops encoding (a/r/w/u). Equivalent to trace remove variable name ops command. Deprecated throughout 8.4-8.6; removed in Tcl 9.0.",
        synopsis: "trace vdelete name ops command",
        return_type: Some(TclType::String),
        mutator: true,
        arg_role_resolver: Some(trace_legacy_arg_roles),
        arg_role_resolver_roles: &[ArgRole::VarWrite],
        command_prefix_resolver: Some(trace_vdelete_command_prefixes),
        script_timing_resolver: Some(trace_vdelete_script_timing),
        arg_values: &[(1, TRACE_LEGACY_OPS_VALUES)],
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        world_effects: Some(LEGACY_TRACE_REMOVE_EFFECTS),
        state_transitions: Some(LEGACY_TRACE_REMOVE_TRANSITIONS),
        literal_argument_validator: Some(validate_legacy_trace_operations),
        // Deprecated legacy form; removed in Tcl 9.0 (8.4-8.6 only) — see
        // `variable` above for why both `dialects` and `lifecycle` state
        // it, and for the measured iRules 8.3-form-only gate.
        surface: Some(surface![
            SpecSurface::core_in(Family::Tcl, &[("8.4", Some("8.7"))]),
            SpecSurface::core(Family::F5Irules)
        ]),
        lifecycle: Lifecycle::deprecated_in("8.4").retired_from("9.0"),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vinfo",
        traits: Traits::TARGETS_VARIABLE_BY_NAME,
        arity: Arity::exact(1),
        detail: "Return trace information for the given variable: one element per trace, each a two-element {ops command} list. Unlike trace info variable, whose first element is a word list, ops here is the legacy single-letter string (a/r/w/u), rendered in the fixed order r, w, u, a — so a trace on reads and writes reports rw. Covers traces added by either spelling. Deprecated throughout 8.4-8.6; removed in Tcl 9.0.",
        synopsis: "trace vinfo name",
        pure: true,
        return_type: Some(TclType::List),
        result_stability: Some(ResultStability::ReadsVersionedWorld(&[
            WorldStateDomain::VariableTraces,
        ])),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        world_effects: Some(LEGACY_TRACE_INFO_EFFECTS),
        state_transitions: Some(StateTransitionDescriptor::EMPTY),
        // Deprecated legacy form; removed in Tcl 9.0 (8.4-8.6 only) — see
        // `variable` above for why both `dialects` and `lifecycle` state
        // it, and for the measured iRules 8.3-form-only gate.
        surface: Some(surface![
            SpecSurface::core_in(Family::Tcl, &[("8.4", Some("8.7"))]),
            SpecSurface::core(Family::F5Irules)
        ]),
        lifecycle: Lifecycle::deprecated_in("8.4").retired_from("9.0"),
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `trace`.
///
/// Per the trace.n manpage fetched for all five versions (8.4, 8.5, 8.6,
/// 9.0, 9.1): the ensemble shape (`add`/`remove`/`info`, each dispatching
/// on a `command`/`execution`/`variable` type) is already present, with
/// this exact wording bar `command` → `commandPrefix` terminology, in Tcl
/// 8.4 — it is not an 8.5+ addition. The only real version boundary is at
/// 9.0, which:
///   - Removes the three deprecated one-word-type subcommands (`trace
///     variable`/`trace vdelete`/`trace vinfo`) entirely — both their
///     SYNOPSIS lines and the whole "For backwards compatibility..."
///     paragraph documenting them are present in 8.4/8.5/8.6 and absent
///     from 9.0/9.1 (byte-identical omission in both). See the
///     `surface: Some(SpecSurface::TCL8X)` gate on `variable`/`vdelete`/
///     `vinfo` below.
///   - Sharpens (without contradicting) the `trace add variable`
///     callback's name1/name2 description: 9.0/9.1 spell out that name2
///     carries the array index even when name1 resolves to what looks
///     like a scalar (possible via `upvar` aliasing a single array
///     element) — folded into the hover snippet below as current,
///     version-neutral fact, since the 8.4-8.6 wording does not
///     contradict it, only under-specifies this one edge case.
///
/// 8.5 and 8.6 are otherwise identical to 8.4 in substance (terminology
/// and copy-editing only), and 9.1's trace.html is byte-for-byte
/// identical to 9.0's (bar the doc-anchor version banner) — no
/// 9.1-specific delta exists for this command.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "trace",
        // Present and unrestricted: `trace` carries an iRules row explicitly
        // (`ALL_TCL.union(IRULES)`), so it resolves under the bare `IRULES`
        // mask, and every dialect that hosts a real Tcl core (irules, iapps,
        // tmsh, the EDA shells, expect, tk, itcl) carries it. Its legal
        // *subcommand* set narrows per Tcl version through each SubCommand's
        // own `dialects` gate below — and, now MEASURED
        // (`docs/design/bigip-irule-parser-measurements.md` §5, BIG-IP
        // 21.1.0.1), per iRules too: TMM's `trace` is the 8.3-era form ONLY.
        // `trace add variable …` fails with `wrong # args`, so the modern
        // `add`/`info`/`remove` subcommands carry `ALL_TCL` (never
        // intersecting the bare `IRULES` mask) while the legacy
        // `variable`/`vdelete`/`vinfo` forms carry `TCL8X.union(IRULES)` — an
        // arity/form gate on the embedded 8.4.6 fork, not a command removal.
        // The `TCL8X` half still extends to every dialect whose
        // `surface_query` composes a real embedded Tcl release with its vendor
        // bit (f5-iapps and f5-tmsh on the fork's 8.4 line, the EDA shells at
        // `TCL85`/`TCL86`, Expect at `TCL86`), per the same intersects-only
        // membership rule `tests/dialect_profile.rs`'s
        // `option_gating_honours_the_version_ceiling` documents.
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        traits: Traits::CREATES_BARRIER | Traits::CREATES_DYNAMIC_BARRIER | Traits::BYTE_COMPILED,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet {
            summary: "Monitor variable accesses, command renames/deletions, and command executions by invoking a callback.",
            synopsis: &["trace option ?arg arg ...?"],
            snippet: "Dispatches on option to add, remove, info, or (deprecated, removed in Tcl 9.0) variable/vdelete/vinfo. trace add type name ops commandPrefix arranges for commandPrefix to be invoked, with operation-specific arguments appended, whenever the variable, command, or command execution named by name undergoes one of the operations in ops; for type command or execution, name must already exist or trace add throws an error, while for type variable a nonexistent name is instead silently created without a value. trace remove undoes a matching trace; trace info returns the traces currently set. type is variable, command, or execution (any unique abbreviation accepted). For a variable trace, ops is one or more of array/read/write/unset, and the callback receives commandPrefix name1 name2 op — name1 is the accessed variable's name and name2 the array index when the trace was set on an array or an array element (present even when name1 itself looks like a scalar, e.g. via an upvar alias to a single element); a read/write callback may itself rewrite the variable to override the traced operation's result, and returning an error from the callback aborts the read/write/access that triggered it; while a read or write callback runs, that same trace is temporarily disabled (so it cannot re-trigger itself), but an unset callback is the one exception — traces stay enabled during unset, so a handler that installs a new trace and touches the variable will see it fire. When several traces are set on the same variable they fire most-recently-added first, stopping at the first one that raises an error, and a trace on the array as a whole fires before one on a single element of it. For a command trace, ops is rename and/or delete, with the callback receiving commandPrefix oldName newName op; deletion cannot be prevented from inside the trace, since Tcl always removes the command once the callback returns, and a rename or delete performed from inside the callback does not re-trigger further traces of that same type. A command trace also never fires when its target disappears because the interpreter itself is being torn down, since there is then no interpreter left to run the callback in. For an execution trace, ops is one or more of enter/leave/enterstep/leavestep — enter/leave fire immediately before/after the traced command itself runs, enterstep/leavestep fire before/after every command nested inside it (meaningful only when name is a procedure) — with the callback receiving commandPrefix command-string op for enter/enterstep or commandPrefix command-string code result op for leave/leavestep; deleting the traced command from inside an enter/enterstep callback stops the pending execution, and while an execution callback runs, that same trace is likewise temporarily disabled. Multiple execution traces on the same name fire in reverse creation order for enter/enterstep and original creation order for leave/leavestep. trace add/remove/variable/vdelete return an empty string; trace info/vinfo return a list of the matching traces (an empty list if none are set), each element itself a two-element {opList commandPrefix} list.",
            source: "Tcl trace(n)",
            examples: "# Log every write to a configuration variable, and read the new value\n# back out via the name(s) the callback receives (not a closure over\n# the original variable name, since upvar can alias it under another).\nproc logWrite {name1 name2 op} {\n    upvar #0 $name1 var\n    puts \"$name1 -> $var\"\n}\ntrace add variable ::config(port) write logWrite\n\n# Trace every command entered while `compute` runs.\nproc traceStep {cmdString op} {\n    puts \"step: $cmdString\"\n}\ntrace add execution compute enterstep traceStep\n\n# Remove a trace again with the exact same arguments used to add it.\ntrace remove variable ::config(port) write logWrite",
            return_value: "Depends on the subcommand: an empty string (add, remove, and the deprecated variable/vdelete), or a list of the matching traces — one element per trace, each itself a two-element {opList commandPrefix} list (info, and the deprecated vinfo).",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        dispatch_dependencies: Some(DispatchDependencyDescriptor::replace(
            DispatchDependencies::NONE,
        )),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use tcl_dialect::model::{Family, SurfaceQuery};

    use super::*;
    use crate::{
        CommandRegistry, InvocationWord, InvocationWordKind, StateTransitionFact,
        StateTransitionWidening,
    };

    #[test]
    fn registry_validates_each_modern_operation_domain_and_builds_a_list_value_fix() {
        let registry = CommandRegistry::build_default();
        for (kind, operations, invalid, allowed, replacement) in [
            (
                "variable",
                "read bogus write",
                "bogus",
                VARIABLE_TRACE_OPERATIONS,
                "read write",
            ),
            (
                "command",
                "delete read rename",
                "read",
                COMMAND_TRACE_OPERATIONS,
                "delete rename",
            ),
            (
                "execution",
                "enter rename leavestep",
                "rename",
                EXECUTION_TRACE_OPERATIONS,
                "enter leavestep",
            ),
        ] {
            let arguments = ["add", kind, "target", operations, "callback"];
            let invocation = registry
                .resolve_invocation(
                    "trace",
                    &arguments,
                    Some(SurfaceQuery::core(Family::Tcl, "9.0")),
                )
                .expect("modern trace form resolves");
            assert_eq!(
                invocation.validate_literal_arguments(),
                Some(LiteralArgumentValidation::Invalid(LiteralArgumentIssue {
                    argument_index: 3,
                    subject: operation_subject(resolve_type(kind).unwrap()),
                    reason: LiteralArgumentIssueReason::InvalidMembers(vec![invalid.to_owned()]),
                    allowed_values: allowed,
                    replacement_value: Some(replacement.to_owned()),
                }))
            );
        }
    }

    #[test]
    fn registry_abstains_on_dynamic_malformed_incomplete_and_invalid_type_operations() {
        let registry = CommandRegistry::build_default();
        let dynamic = [
            InvocationWord::Literal("add"),
            InvocationWord::Literal("variable"),
            InvocationWord::Literal("target"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("callback"),
        ];
        let invocation = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(InvocationWord::Literal("trace"), &dynamic),
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .resolved()
            .expect("dynamic one-word argument keeps the invocation resolvable");
        assert_eq!(
            invocation.validate_literal_arguments(),
            Some(LiteralArgumentValidation::Abstain(
                LiteralValidationDecline::NonLiteralArgument
            ))
        );

        for (arguments, decline) in [
            (
                &["add", "variable", "target", "{unterminated", "callback"][..],
                LiteralValidationDecline::MalformedLiteral,
            ),
            (
                &["add", "variable", "target", "read"][..],
                LiteralValidationDecline::IncompleteInvocation,
            ),
            (
                &["add", "not-a-type", "target", "read", "callback"][..],
                LiteralValidationDecline::InvalidDiscriminator,
            ),
        ] {
            let invocation = registry
                .resolve_invocation(
                    "trace",
                    arguments,
                    Some(SurfaceQuery::core(Family::Tcl, "9.0")),
                )
                .expect("literal trace head resolves even when an argument is invalid");
            assert_eq!(
                invocation.validate_literal_arguments(),
                Some(LiteralArgumentValidation::Abstain(decline))
            );
        }
    }

    #[test]
    fn empty_and_all_invalid_modern_lists_never_offer_an_invalid_empty_replacement() {
        let registry = CommandRegistry::build_default();
        for operations in ["", "bogus nope"] {
            let arguments = ["remove", "command", "target", operations, "callback"];
            let invocation = registry
                .resolve_invocation(
                    "trace",
                    &arguments,
                    Some(SurfaceQuery::core(Family::Tcl, "9.0")),
                )
                .expect("modern trace remove resolves");
            let Some(LiteralArgumentValidation::Invalid(issue)) =
                invocation.validate_literal_arguments()
            else {
                panic!("empty/all-invalid operation list must be invalid");
            };
            assert_eq!(issue.replacement_value, None);
        }
    }

    #[test]
    fn legacy_validator_exists_only_on_tcl8_forms() {
        let registry = CommandRegistry::build_default();
        for dialect in [
            Some(SurfaceQuery::core(Family::Tcl, "8.4")),
            Some(SurfaceQuery::core(Family::Tcl, "8.5")),
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        ] {
            let invocation = registry
                .resolve_invocation("trace", &["variable", "target", "rwx", "callback"], dialect)
                .expect("legacy trace variable resolves in Tcl 8.x");
            let Some(LiteralArgumentValidation::Invalid(issue)) =
                invocation.validate_literal_arguments()
            else {
                panic!("invalid legacy operation character must be reported");
            };
            assert_eq!(
                issue.reason,
                LiteralArgumentIssueReason::InvalidMembers(vec!["x".to_owned()])
            );
            assert_eq!(issue.allowed_values, LEGACY_VARIABLE_TRACE_OPERATIONS);
            assert_eq!(issue.replacement_value, None);
        }
        assert!(
            registry
                .resolve_invocation(
                    "trace",
                    &["variable", "target", "rwx", "callback"],
                    Some(SurfaceQuery::core(Family::Tcl, "9.0")),
                )
                .and_then(|invocation| invocation.validate_literal_arguments())
                .is_none(),
            "the removed legacy subcommand must not acquire a Tcl 9 validator"
        );
    }

    #[test]
    fn execution_callback_arity_tracks_each_literal_operation_domain() {
        let registry = CommandRegistry::build_default();
        for (operations, expected) in [
            ("enter enterstep", AppendedArity::Exactly(2)),
            ("leave leavestep", AppendedArity::Exactly(4)),
            (
                "enter leavestep",
                AppendedArity::OneOf(EXECUTION_TRACE_CALLBACK_ARITIES),
            ),
        ] {
            assert_eq!(
                registry.command_prefixes(
                    "trace",
                    &["add", "execution", "target", operations, "callback"]
                ),
                vec![(4, expected)],
                "unexpected execution callback contract for {operations:?}"
            );
        }
    }

    #[test]
    fn execution_callback_arity_abstains_on_unproved_operation_lists() {
        let registry = CommandRegistry::build_default();
        for operations in ["{enter", "", "enter invalid"] {
            assert_eq!(
                registry.command_prefixes(
                    "trace",
                    &["add", "execution", "target", operations, "callback"]
                ),
                vec![(4, AppendedArity::Unknown)],
                "malformed/empty/invalid operation lists must abstain"
            );
        }

        let spellings = ["add", "execution", "target", "$operations", "callback"];
        let words = [
            InvocationWord::Literal("add"),
            InvocationWord::Literal("execution"),
            InvocationWord::Literal("target"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("callback"),
        ];
        assert_eq!(
            registry.command_prefixes_structured("trace", &spellings, &words),
            vec![(4, AppendedArity::Unknown)],
            "a substituted operation list is not its source spelling"
        );
    }

    #[test]
    fn modern_variable_add_canonicalises_duplicate_operations() {
        let transitions = TRACE_ADD_TRANSITIONS.resolve(InvocationArguments::literals(&[
            "add",
            "var",
            "item",
            "write read write unset",
            "{callback one}",
        ]));

        assert_eq!(
            transitions.facts(),
            &[StateTransitionFact {
                transition: StateTransition::Trace(TraceTransition::Add {
                    target: TraceTarget::Variable(TransitionSubject::Literal("item".to_owned())),
                    // The canonical order is C's `trace info` render order
                    // (`array read write unset`), not the `opStrings[]` table
                    // order the bad-operation error enumerates.
                    operations: TraceOperationSet::Known(vec![
                        TraceOperation::Read,
                        TraceOperation::Write,
                        TraceOperation::Unset,
                    ]),
                    prefix: TransitionSubject::Literal("{callback one}".to_owned()),
                }),
                commit: StateTransitionCommit::OnOkOnly,
            }]
        );
    }

    #[test]
    fn dynamic_one_word_op_list_preserves_a_typed_trace_transition() {
        let arguments = [
            InvocationWord::Literal("add"),
            InvocationWord::Literal("command"),
            InvocationWord::Literal("worker"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("prefix"),
        ];
        let transitions =
            TRACE_ADD_TRANSITIONS.resolve(InvocationArguments::structured(&arguments));
        assert!(matches!(
            transitions.facts(),
            [StateTransitionFact {
                transition: StateTransition::Trace(TraceTransition::Add {
                    target: TraceTarget::Command(TransitionSubject::Literal(target)),
                    operations: TraceOperationSet::Unknown(TransitionSubject::Unknown {
                        argument_index: 3,
                        word_kind: InvocationWordKind::Dynamic,
                    }),
                    ..
                }),
                commit: StateTransitionCommit::OnOkOnly,
            }] if target == "worker"
        ));
    }

    #[test]
    fn add_and_remove_widen_only_their_distinct_trace_unions() {
        let dynamic_type = [
            InvocationWord::Literal("add"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("target"),
            InvocationWord::Literal("read"),
            InvocationWord::Literal("prefix"),
        ];
        let dynamic_remove_type = [
            InvocationWord::Literal("remove"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("target"),
            InvocationWord::Literal("read"),
            InvocationWord::Literal("prefix"),
        ];
        let expanded_prefix = [
            InvocationWord::Literal("add"),
            InvocationWord::Literal("variable"),
            InvocationWord::Literal("target"),
            InvocationWord::Literal("read"),
            InvocationWord::Expanded,
        ];
        for (descriptor, arguments, expected_domains, expected_subject) in [
            (
                TRACE_ADD_TRANSITIONS,
                InvocationArguments::structured(&dynamic_type),
                TRACE_ADD_TRANSITION_DOMAINS,
                TransitionSubject::Unknown {
                    argument_index: 1,
                    word_kind: InvocationWordKind::Dynamic,
                },
            ),
            (
                TRACE_REMOVE_TRANSITIONS,
                InvocationArguments::structured(&dynamic_remove_type),
                TRACE_REMOVE_TRANSITION_DOMAINS,
                TransitionSubject::Unknown {
                    argument_index: 1,
                    word_kind: InvocationWordKind::Dynamic,
                },
            ),
            (
                TRACE_ADD_TRANSITIONS,
                InvocationArguments::structured(&expanded_prefix),
                TRACE_ADD_TRANSITION_DOMAINS,
                TransitionSubject::Unknown {
                    argument_index: 4,
                    word_kind: InvocationWordKind::Expanded,
                },
            ),
        ] {
            let transitions = descriptor.resolve(arguments);
            assert_eq!(
                transitions.facts(),
                &[StateTransitionFact {
                    transition: StateTransition::Widen(StateTransitionWidening {
                        domains: expected_domains.to_vec(),
                        subject: expected_subject,
                    }),
                    commit: StateTransitionCommit::OnOkOnly,
                }]
            );
        }
    }

    #[test]
    fn dynamic_prefix_has_no_immediate_callback_and_info_fallback_is_read_only() {
        let literal = TRACE_ADD_EFFECTS.resolve(InvocationArguments::literals(&[
            "add",
            "execution",
            "worker",
            "enter",
            "prefix",
        ]));
        assert_eq!(literal.callback(), CallbackEffect::NONE);
        assert!(literal.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::ExecutionTraces
                && access.mode == EffectAccessMode::ReadWrite
        }));

        let dynamic_prefix = [
            InvocationWord::Literal("add"),
            InvocationWord::Literal("variable"),
            InvocationWord::Literal("item"),
            InvocationWord::Literal("write"),
            InvocationWord::Dynamic,
        ];
        let dynamic = TRACE_ADD_EFFECTS.resolve(InvocationArguments::structured(&dynamic_prefix));
        assert_eq!(dynamic.callback(), CallbackEffect::NONE);
        assert!(dynamic.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::CommandTraces
                && access.mode == EffectAccessMode::ReadWrite
        }));

        let dynamic_info_type = [
            InvocationWord::Literal("info"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("target"),
        ];
        let info = TRACE_INFO_EFFECTS.resolve(InvocationArguments::structured(&dynamic_info_type));
        assert_eq!(info.callback(), CallbackEffect::NONE);
        assert!(
            info.accesses()
                .iter()
                .all(|access| access.mode == EffectAccessMode::Read)
        );
    }

    #[test]
    fn resolved_trace_add_keeps_reads_but_commits_writes_on_the_ok_edge() {
        let registry = CommandRegistry::build_default();
        let facts = registry
            .resolve_invocation(
                "trace",
                &["add", "variable", "item", "write", "prefix"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("literal trace add resolves")
            .facts();

        assert!(facts.effects.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::VariableTraces
                && access.mode == EffectAccessMode::Read
        }));
        assert!(facts.effects.accesses().iter().all(|access| {
            !matches!(
                access.domain,
                WorldStateDomain::VariableStore | WorldStateDomain::VariableTraces
            ) || access.mode == EffectAccessMode::Read
        }));
        assert!(matches!(
            facts
                .state_transitions
                .declared()
                .map(StateTransitions::facts),
            Some([StateTransitionFact {
                transition: StateTransition::Trace(TraceTransition::Add { .. }),
                commit: StateTransitionCommit::OnOkOnly,
            }])
        ));
    }

    #[test]
    fn oracle_profiles_keep_legacy_forms_in_tcl8_only() {
        let registry = CommandRegistry::build_default();
        for dialect in [
            Some(SurfaceQuery::core(Family::Tcl, "8.4")),
            Some(SurfaceQuery::core(Family::Tcl, "8.5")),
            Some(SurfaceQuery::core(Family::Tcl, "8.6")),
        ] {
            for (arguments, expected) in [
                (&["variable", "item", "rw", "prefix"][..], "variable"),
                (&["vdelete", "item", "rw", "prefix"][..], "vdelete"),
                (&["vinfo", "item"][..], "vinfo"),
            ] {
                let canonical = registry
                    .resolve_invocation("trace", arguments, dialect)
                    .and_then(|invocation| {
                        invocation
                            .facts()
                            .subcommand
                            .canonical_name()
                            .map(str::to_owned)
                    });
                assert_eq!(canonical.as_deref(), Some(expected));
            }
        }
        for dialect in [
            Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            Some(SurfaceQuery::core(Family::Tcl, "9.1")),
        ] {
            for arguments in [
                &["variable", "item", "rw", "prefix"][..],
                &["vdelete", "item", "rw", "prefix"][..],
                &["vinfo", "item"][..],
            ] {
                let canonical = registry
                    .resolve_invocation("trace", arguments, dialect)
                    .and_then(|invocation| {
                        invocation
                            .facts()
                            .subcommand
                            .canonical_name()
                            .map(str::to_owned)
                    });
                assert_eq!(canonical, None);
            }
        }
    }

    #[test]
    fn legacy_forms_resolve_to_the_same_typed_variable_trace_domains() {
        let registry = CommandRegistry::build_default();
        let add = registry
            .resolve_invocation(
                "trace",
                &["variable", "item", "awrw", "prefix"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("legacy add resolves")
            .facts();
        let remove = registry
            .resolve_invocation(
                "trace",
                &["vdelete", "item", "awrw", "prefix"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("legacy remove resolves")
            .facts();

        for (facts, expected) in [(&add, true), (&remove, false)] {
            assert!(facts.effects.accesses().iter().any(|access| {
                access.domain == WorldStateDomain::VariableStore
                    && access.mode == EffectAccessMode::Read
            }));
            assert!(facts.effects.accesses().iter().any(|access| {
                access.domain == WorldStateDomain::VariableTraces
                    && access.mode == EffectAccessMode::Read
            }));
            let declared = facts
                .state_transitions
                .declared()
                .expect("legacy transition is declared");
            assert!(
                matches!(
                    declared.facts(),
                    [StateTransitionFact {
                        transition: StateTransition::Trace(TraceTransition::Add {
                            target: TraceTarget::Variable(TransitionSubject::Literal(target)),
                            operations: TraceOperationSet::Known(operations),
                            ..
                        }),
                        ..
                    }] if expected && target == "item" && operations.as_slice() == [
                        TraceOperation::Array,
                        TraceOperation::Read,
                        TraceOperation::Write,
                    ]
                ) || matches!(
                    declared.facts(),
                    [StateTransitionFact {
                        transition: StateTransition::Trace(TraceTransition::Remove {
                            target: TraceTarget::Variable(TransitionSubject::Literal(target)),
                            operations: TraceOperationSet::Known(operations),
                            ..
                        }),
                        ..
                    }] if !expected && target == "item" && operations.as_slice() == [
                        TraceOperation::Array,
                        TraceOperation::Read,
                        TraceOperation::Write,
                    ]
                )
            );
        }
    }

    #[test]
    fn trace_info_results_and_dispatch_are_registry_versioned() {
        let registry = CommandRegistry::build_default();
        let facts = registry
            .resolve_invocation(
                "trace",
                &["info", "execution", "llength"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .expect("trace info resolves")
            .facts();

        assert_eq!(
            facts.result_stability,
            ResultStability::ReadsVersionedWorld(TRACE_INFO_RESULT_DOMAINS)
        );
        assert_eq!(facts.dispatch_dependencies, DispatchDependencies::BASE);
    }

    #[test]
    fn removing_a_variable_trace_touches_the_placeholder_cell() {
        let registry = CommandRegistry::build_default();
        let facts = registry
            .resolve_invocation(
                "trace",
                &["remove", "variable", "item", "write", "prefix"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .expect("trace remove resolves")
            .facts();

        assert!(facts.effects.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::VariableStore
                && access.mode == EffectAccessMode::Read
        }));
        assert!(matches!(
            facts
                .state_transitions
                .declared()
                .map(StateTransitions::facts),
            Some([StateTransitionFact {
                transition: StateTransition::Trace(TraceTransition::Remove {
                    target: TraceTarget::Variable(_),
                    ..
                }),
                commit: StateTransitionCommit::OnOkOnly,
            }])
        ));
    }
}
