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

//! `oo::copy` — create copies of `TclOO` objects and classes.
//
// TclOO merged into Tcl core at 8.6: tcl-lang.org's 8.4 and 8.5 TclCmd
// trees have no copy.n page (a soft-404 "URL Not Found" page under a 200
// status) — confirmed absent, not merely undocumented. The 8.6, 9.0, and
// 9.1 manpages document an identical `oo::copy sourceObject ?targetObject?
// ?targetNamespace?` synopsis, DESCRIPTION, and EXAMPLES section; the only
// textual differences across them are cosmetic — the SYNOPSIS's `package
// require` line (`TclOO` in 8.6, `tcl::oo` from 9.0 on) and the NAME
// line's capitalisation ("create copies…" in 8.6, "Create copies…" from
// 9.0 on) — neither affects this command's own arguments or behaviour.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const COPY_SOURCE_TRANSITION_DOMAINS: &[StateTransitionDomain] = &[
    StateTransitionDomain::ObjectDispatch,
    StateTransitionDomain::VariableCells,
];

const COPY_TARGET_TRANSITION_DOMAINS: &[StateTransitionDomain] = &[
    StateTransitionDomain::CommandBindings,
    StateTransitionDomain::Namespaces,
    StateTransitionDomain::VariableCells,
    StateTransitionDomain::ObjectDispatch,
];

const COPY_NAMESPACE_TRANSITION_DOMAINS: &[StateTransitionDomain] = &[
    StateTransitionDomain::Namespaces,
    StateTransitionDomain::VariableCells,
    StateTransitionDomain::ObjectDispatch,
];

const COPY_INTERP_EFFECT_COVERAGE: &[TransitionEffectCoverage] = &[TransitionEffectCoverage {
    source: WorldEffectWriteSource::LegacySideEffect(SideEffectTarget::InterpState),
    domains: &[WorldStateDomain::InterpreterPolicy],
}];

const COPY_EFFECTS: WorldEffectDescriptor = WorldEffectDescriptor {
    composition: WorldEffectComposition::Extend,
    static_footprint: StaticEffectFootprint {
        accesses: &[],
        // `<cloned>` is ordinary user-overridable TclOO method dispatch. The
        // copy is live during it, but a non-OK completion deletes the copy
        // before `oo::copy` propagates that completion.
        callback: CallbackEffect {
            kinds: CallbackKinds::SCRIPT,
            reentrancy: Reentrancy::CurrentInterpreter,
        },
    },
    resolver: None,
    dynamic_fallback: WorldEffectDynamicFallback::ConservativeUnknownInvocation,
};

const COPY_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(copy_state_transitions),
    argument_shape: StateTransitionArgumentShape::Positional,
    dynamic_widening: &[
        StateTransitionWideningRule {
            operands: StateTransitionOperandLayout::Indices(&[0]),
            domains: COPY_SOURCE_TRANSITION_DOMAINS,
        },
        StateTransitionWideningRule {
            operands: StateTransitionOperandLayout::Indices(&[1]),
            domains: COPY_TARGET_TRANSITION_DOMAINS,
        },
        StateTransitionWideningRule {
            operands: StateTransitionOperandLayout::Indices(&[2]),
            domains: COPY_NAMESPACE_TRANSITION_DOMAINS,
        },
    ],
    effect_coverage: COPY_INTERP_EFFECT_COVERAGE,
    commit: StateTransitionCommit::OnOkOnly,
};

fn copy_target(subject: Option<TransitionSubject>) -> ObjectDispatchTarget {
    match subject {
        None => ObjectDispatchTarget::Fresh,
        Some(TransitionSubject::Literal(ref name)) if name.is_empty() => {
            ObjectDispatchTarget::Fresh
        }
        Some(subject) => ObjectDispatchTarget::Named(subject),
    }
}

fn copy_private_namespace(subject: Option<TransitionSubject>) -> ObjectPrivateNamespace {
    match subject {
        None => ObjectPrivateNamespace::Fresh,
        Some(TransitionSubject::Literal(ref name)) if name.is_empty() => {
            ObjectPrivateNamespace::Fresh
        }
        Some(subject) => ObjectPrivateNamespace::Named(subject),
    }
}

fn copy_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    let Some(source) = TransitionSubject::from_argument(arguments, 0) else {
        return transitions;
    };
    let target_subject = TransitionSubject::from_argument(arguments, 1);
    if let Some(target @ TransitionSubject::Literal(_)) = target_subject.as_ref()
        && target.literal().is_some_and(|name| !name.is_empty())
    {
        transitions.push(StateTransition::CommandBinding(
            CommandBindingTransition::Define {
                name: target.clone(),
                kind: CommandBindingDefinitionKind::Object,
            },
        ));
    }
    transitions.push(StateTransition::ObjectDispatch(
        ObjectDispatchTransition::Copy {
            source,
            target: copy_target(target_subject),
            private_namespace: copy_private_namespace(TransitionSubject::from_argument(
                arguments, 2,
            )),
        },
    ));
    transitions
}

// oo::copy both (a) reads sourceObject's existing class/method/procedure/
// variable state to duplicate it and (b) writes the copy's command-table
// entry and installed configuration — a read-then-write pair, like
// `HTTP::header replace`'s. It also synchronously invokes the copy's
// `<cloned>` method before returning; the default implementation (in
// oo::object) only copies procedures/variables (still InterpState), but
// any class may override `<cloned>` to do anything an ordinary method
// body can do, so — mirroring `coroutine`'s own synchronously-run
// callback — that unknowable portion is folded into this command's own
// side effects rather than deferred to a callee.
const SIDE_EFFECTS: &[SideEffect] = &[
    SideEffect {
        target: SideEffectTarget::InterpState,
        reads: true,
        writes: true,
        ..SideEffect::DEFAULT
    },
    SideEffect {
        ..SideEffect::DEFAULT
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "oo::copy sourceObject ?targetObject? ?targetNamespace?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::copy",
        surface: Some(SpecSurface::TCL86_PLUS),
        // Three positional arguments total (sourceObject required,
        // targetObject and targetNamespace each optional), matching the
        // synopsis below in every version that has this command (8.6,
        // 9.0, 9.1) — a prior `Arity::new(1, 2)` wrongly rejected the
        // fully-qualified `oo::copy $src $dst $ns` form.
        arity: Arity::new(1, 3),
        return_type: Some(TclType::String),
        // `oo::copy sourceObject targetObject` binds targetObject as the
        // copy's command (`targetObject <method> …` then dispatches on
        // it) when given literally. Index 1 is after sourceObject; an
        // omitted or empty-string targetObject — auto-generated at run
        // time — is skipped by the consumer, the same optional-name shape
        // as `interp create`'s subcommand spec.
        defines_command_at: Some(1),
        hover: Some(HoverSnippet {
            summary: "create copies of objects and classes",
            synopsis: &["oo::copy sourceObject ?targetObject? ?targetNamespace?"],
            snippet: "Duplicates sourceObject, an existing TclOO object or class. The copy is of the same class as sourceObject and gets all of its per-object methods; if sourceObject is a class, its class methods are copied too, but its instances are not. targetObject names the new command (resolved relative to the current namespace unless already fully qualified) and targetNamespace names the namespace holding its internal state (the `my` command's scope, etc.), which must not already exist. Omitting either, or passing it as the empty string, picks an automatically generated name using the same algorithm as oo::class's new method. Once the copy's configuration (methods, filters, mixins, …) has been installed, its <cloned> method is invoked with sourceObject as its sole argument, so a class can customise what a copy of one of its instances looks like — for example installing its own variable traces. The default <cloned> implementation, defined on oo::object, just copies the procedures and variables from sourceObject's namespace into the copy's. If that call raises an error or other exception, the half-built copy is deleted and the exception propagates from oo::copy.",
            source: "Tcl man page copy.n",
            examples: "oo::object create src\noo::objdefine src method msg {} {puts foo}\noo::copy src dst\noo::objdefine src method msg {} {puts bar}\nsrc msg    ;# bar -- src was redefined after the copy\ndst msg    ;# foo -- dst kept the method as it stood when copied",
            return_value: "The fully-qualified name of the newly created object or class.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        world_effects: Some(COPY_EFFECTS),
        state_transitions: Some(COPY_TRANSITIONS),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use tcl_dialect::model::{Family, SurfaceQuery};

    use crate::{
        CallbackKinds, CommandBindingTransition, CommandRegistry, EffectAccessMode,
        ObjectDispatchTarget, ObjectDispatchTransition, ObjectPrivateNamespace, StateTransition,
        StateTransitionCommit, WorldStateDomain,
    };

    #[test]
    fn copy_retains_source_target_and_private_namespace_lifecycle() {
        let registry = CommandRegistry::build_default();
        let invocation = registry
            .resolve_invocation(
                "oo::copy",
                &["::source", "::target", "::private::target"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("oo::copy must resolve");
        let transitions = invocation.state_transitions();
        assert!(
            transitions
                .facts()
                .iter()
                .all(|fact| fact.commit == StateTransitionCommit::OnOkOnly)
        );
        assert!(transitions.facts().iter().any(|fact| matches!(
            &fact.transition,
            StateTransition::CommandBinding(CommandBindingTransition::Define { name, .. })
                if name.literal() == Some("::target")
        )));
        assert!(transitions.facts().iter().any(|fact| matches!(
            &fact.transition,
            StateTransition::ObjectDispatch(ObjectDispatchTransition::Copy {
                source,
                target: ObjectDispatchTarget::Named(target),
                private_namespace: ObjectPrivateNamespace::Named(namespace),
            }) if source.literal() == Some("::source")
                && target.literal() == Some("::target")
                && namespace.literal() == Some("::private::target")
        )));

        let effects = invocation.effect_footprint();
        assert!(!effects.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::InterpreterPolicy
                && matches!(
                    access.mode,
                    EffectAccessMode::Write
                        | EffectAccessMode::ReadWrite
                        | EffectAccessMode::Clobber
                )
        }));
        assert!(effects.callback().kinds.contains(CallbackKinds::SCRIPT));
    }

    #[test]
    fn omitted_copy_identities_remain_typed_fresh_values() {
        let registry = CommandRegistry::build_default();
        let transitions = registry
            .resolve_invocation(
                "oo::copy",
                &["::source"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("oo::copy must resolve")
            .state_transitions();
        assert!(transitions.facts().iter().any(|fact| matches!(
            &fact.transition,
            StateTransition::ObjectDispatch(ObjectDispatchTransition::Copy {
                target: ObjectDispatchTarget::Fresh,
                private_namespace: ObjectPrivateNamespace::Fresh,
                ..
            })
        )));
    }
}
