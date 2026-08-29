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

//! `oo::class` — the class of all classes; the `TclOO` metaclass.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const CLASS_NAMED_CREATE_TRANSITION_DOMAINS: &[StateTransitionDomain] = &[
    StateTransitionDomain::CommandBindings,
    StateTransitionDomain::Namespaces,
    StateTransitionDomain::ObjectDispatch,
];

const CLASS_EXPLICIT_NAMESPACE_TRANSITION_DOMAINS: &[StateTransitionDomain] = &[
    StateTransitionDomain::Namespaces,
    StateTransitionDomain::VariableCells,
    StateTransitionDomain::ObjectDispatch,
];

const CLASS_INTERP_EFFECT_COVERAGE: &[TransitionEffectCoverage] = &[TransitionEffectCoverage {
    source: WorldEffectWriteSource::LegacySideEffect(SideEffectTarget::InterpState),
    domains: &[WorldStateDomain::InterpreterPolicy],
}];

const CLASS_FACTORY_EFFECTS: WorldEffectDescriptor = WorldEffectDescriptor {
    composition: WorldEffectComposition::Extend,
    static_footprint: StaticEffectFootprint {
        accesses: &[],
        // A definition body runs while the new class is visible, but any
        // non-OK completion rolls the class, command, and private namespace
        // back before it escapes from the factory.
        callback: CallbackEffect {
            kinds: CallbackKinds::SCRIPT,
            reentrancy: Reentrancy::CurrentInterpreter,
        },
    },
    resolver: None,
    dynamic_fallback: WorldEffectDynamicFallback::ConservativeUnknownInvocation,
};

const CLASS_CREATE_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(class_create_state_transitions),
    argument_shape: StateTransitionArgumentShape::Positional,
    dynamic_widening: &[StateTransitionWideningRule {
        operands: StateTransitionOperandLayout::Indices(&[1]),
        domains: CLASS_NAMED_CREATE_TRANSITION_DOMAINS,
    }],
    effect_coverage: CLASS_INTERP_EFFECT_COVERAGE,
    commit: StateTransitionCommit::OnOkOnly,
};

const CLASS_NEW_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(class_new_state_transitions),
    argument_shape: StateTransitionArgumentShape::Independent,
    dynamic_widening: &[],
    effect_coverage: CLASS_INTERP_EFFECT_COVERAGE,
    commit: StateTransitionCommit::OnOkOnly,
};

const CLASS_CREATE_WITH_NAMESPACE_TRANSITIONS: StateTransitionDescriptor =
    StateTransitionDescriptor {
        composition: StateTransitionComposition::Extend,
        resolver: Some(class_create_with_namespace_state_transitions),
        argument_shape: StateTransitionArgumentShape::Positional,
        dynamic_widening: &[
            StateTransitionWideningRule {
                operands: StateTransitionOperandLayout::Indices(&[1]),
                domains: CLASS_NAMED_CREATE_TRANSITION_DOMAINS,
            },
            StateTransitionWideningRule {
                operands: StateTransitionOperandLayout::Indices(&[2]),
                domains: CLASS_EXPLICIT_NAMESPACE_TRANSITION_DOMAINS,
            },
        ],
        effect_coverage: CLASS_INTERP_EFFECT_COVERAGE,
        commit: StateTransitionCommit::OnOkOnly,
    };

fn push_named_class_creation(
    transitions: &mut StateTransitions,
    target: TransitionSubject,
    private_namespace: ObjectPrivateNamespace,
) {
    if matches!(&target, TransitionSubject::Literal(name) if !name.is_empty()) {
        transitions.push(StateTransition::CommandBinding(
            CommandBindingTransition::Define {
                name: target.clone(),
                kind: CommandBindingDefinitionKind::Object,
            },
        ));
    }
    transitions.push(StateTransition::ObjectDispatch(
        ObjectDispatchTransition::Create {
            target: ObjectDispatchTarget::Named(target),
            private_namespace,
            kind: ObjectDispatchKind::Class,
        },
    ));
}

fn class_create_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    if let Some(target) = TransitionSubject::from_argument(arguments, 1) {
        push_named_class_creation(&mut transitions, target, ObjectPrivateNamespace::Fresh);
    }
    transitions
}

fn class_new_state_transitions(_arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    transitions.push(StateTransition::ObjectDispatch(
        ObjectDispatchTransition::Create {
            target: ObjectDispatchTarget::Fresh,
            private_namespace: ObjectPrivateNamespace::Fresh,
            kind: ObjectDispatchKind::Class,
        },
    ));
    transitions
}

fn class_create_with_namespace_state_transitions(
    arguments: InvocationArguments<'_>,
) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    let (Some(target), Some(private_namespace)) = (
        TransitionSubject::from_argument(arguments, 1),
        TransitionSubject::from_argument(arguments, 2),
    ) else {
        return transitions;
    };
    push_named_class_creation(
        &mut transitions,
        target,
        ObjectPrivateNamespace::Named(private_namespace),
    );
    transitions
}

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "oo::class method ?arg ...?",
    ..FormSpec::DEFAULT
}];

/// The three class-manufacturing methods `class.n` documents for
/// `oo::class` — non-exhaustive completion/hover data only, never
/// `closed_value_args`: `oo::class` is itself an ordinary `TclOO`
/// object, so it also answers to every method `oo::object` defines
/// (`destroy`, …) and to any method grafted on afterwards (e.g. via
/// `oo::objdefine`), neither of which belongs in a fixed enum here.
const CLASS_METHOD_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "create",
        detail: "Create a new instance named name, passing the optional definition script to oo::define. Returns the fully qualified object name.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "new",
        detail: "Create a new instance with an automatically generated unique name. Not exported on oo::class itself — new classes should be made with create.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "createWithNamespace",
        detail: "Non-exported: like create, but the instance's internal namespace is explicitly named nsName instead of being auto-generated.",
        ..ArgValue::DEFAULT
    },
];

pub(crate) const CLASS_FACTORY_SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "create",
        arity: Arity::new(1, 2),
        detail: "Create a named class, optionally evaluating a definition body.",
        synopsis: "oo::class create name ?definition?",
        arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Body)],
        body_kind: BodyKind::Structural,
        return_type: Some(TclType::String),
        defines_command_at: Some(0),
        world_effects: Some(CLASS_FACTORY_EFFECTS),
        state_transitions: Some(CLASS_CREATE_TRANSITIONS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "new",
        arity: Arity::new(0, 1),
        detail: "Create an automatically named class, optionally evaluating a definition body.",
        synopsis: "oo::class new ?definition?",
        arg_roles: &[(0, ArgRole::Body)],
        body_kind: BodyKind::Structural,
        return_type: Some(TclType::String),
        world_effects: Some(CLASS_FACTORY_EFFECTS),
        state_transitions: Some(CLASS_NEW_TRANSITIONS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "createWithNamespace",
        arity: Arity::new(2, 3),
        detail: "Create a named class with an explicitly named private namespace.",
        synopsis: "oo::class createWithNamespace name nsName ?definition?",
        arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Name), (2, ArgRole::Body)],
        body_kind: BodyKind::Structural,
        return_type: Some(TclType::String),
        defines_command_at: Some(0),
        world_effects: Some(CLASS_FACTORY_EFFECTS),
        state_transitions: Some(CLASS_CREATE_WITH_NAMESPACE_TRANSITIONS),
        ..SubCommand::DEFAULT
    },
];

/// Resolve the body argument index for the metaclass shapes:
///
/// * `oo::class create Name body` → body at index 2.
/// * `oo::class new body` → body at index 1.
/// * `oo::class createWithNamespace Name ::ns body` → body at index 3.
///
/// Shared by every `IS_OO_METACLASS` command (`oo::class`,
/// `oo::configurable`, `oo::abstract`, `oo::singleton`) — they all
/// manufacture classes with the same `create` / `new` /
/// `createWithNamespace` shapes, so their definition-body argument
/// lives at the same index.
pub(crate) fn oo_class_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let Some(body) = args
        .first()
        .and_then(|word| crate::definer::TCLOO_GRAMMAR.manufacturer(word))
        .and_then(|method| method.definition_body_at)
    else {
        return Vec::new();
    };
    (usize::from(body) < args.len())
        .then_some(vec![(body, ArgRole::Body)])
        .unwrap_or_default()
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::class",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::IS_OO_METACLASS
            | Traits::LANGUAGE_KEYWORD
            | Traits::DEFINES_PROCEDURE,
        surface: Some(SpecSurface::TCL86_PLUS),
        arity: Arity::at_least(1),
        arg_role_resolver: Some(oo_class_arg_roles),
        return_type: Some(TclType::String),
        subcommands: CLASS_FACTORY_SUBCOMMANDS,
        allow_unknown_subcommands: true,
        // Bodies of `oo::class create / new / createWithNamespace`
        // run in a TclOO definition context (not the caller's frame).
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "class of all classes",
            synopsis: &[
                "oo::class method ?arg ...?",
                "oo::class create name ?definition?",
                "oo::class new ?definition?",
                "oo::class createWithNamespace name nsName ?definition?",
            ],
            snippet: "Classes are objects that manufacture other objects according to a pattern stored in the factory object (the class): create makes an instance with an explicit name, new makes one with an automatically generated unique name. oo::class is the class of all classes — every class, including oo::class itself, is an instance of oo::class, and oo::class is a subclass of oo::object, so every class is also an object. oo::class hides its own new method on itself, so a brand-new class should always be made with create; a class made that way still exports a normal, working new of its own for creating instances. The constructor takes a single optional argument, passed on to oo::define together with the new class's name, so a class body can be supplied directly at creation time. There is no explicit destructor: destroying a class also destroys all its subclasses, instances, and any objects it has been mixed into. The non-exported createWithNamespace method behaves like create but additionally lets the caller name the instance's internal namespace explicitly.",
            source: "Tcl man page class.n",
            examples: "oo::class create fruit {\n    method eat {} {\n        puts \"yummy!\"\n    }\n}\noo::class create banana {\n    superclass fruit\n    constructor {} {\n        my variable peeled\n        set peeled 0\n    }\n    method peel {} {\n        my variable peeled\n        set peeled 1\n        puts \"skin now off\"\n    }\n    method edible? {} {\n        my variable peeled\n        return $peeled\n    }\n    method eat {} {\n        if {![my edible?]} {\n            my peel\n        }\n        next\n    }\n}\nset b [banana new]\n$b eat    ;# prints \"skin now off\" and \"yummy!\"\nfruit destroy\n$b eat    ;# error \"unknown command\"",
            return_value: "The fully qualified name of the newly created class for create, new, and createWithNamespace; otherwise whatever the invoked method returns.",
        }),
        forms: FORMS,
        arg_values: &[(0, CLASS_METHOD_VALUES)],
        side_effects: SIDE_EFFECTS,
        definition_body: Some(&crate::definer::TCLOO_GRAMMAR),
        manufacturer_methods: crate::definer::TCLOO_ROOT_CLASS_MANUFACTURERS,
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use tcl_dialect::model::{Family, SurfaceQuery};

    use crate::{
        CallbackKinds, CommandBindingTransition, CommandRegistry, ObjectDispatchKind,
        ObjectDispatchTarget, ObjectDispatchTransition, ObjectPrivateNamespace, StateTransition,
        StateTransitionCommit, WorldStateDomain,
    };

    #[test]
    fn class_create_records_command_dispatch_and_independent_private_namespace() {
        let registry = CommandRegistry::build_default();
        let invocation = registry
            .resolve_invocation(
                "oo::class",
                &["create", "::C", "{}"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("oo::class create must resolve");
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
                if name.literal() == Some("::C")
        )));
        assert!(transitions.facts().iter().any(|fact| matches!(
            &fact.transition,
            StateTransition::ObjectDispatch(ObjectDispatchTransition::Create {
                target: ObjectDispatchTarget::Named(target),
                private_namespace: ObjectPrivateNamespace::Fresh,
                kind: ObjectDispatchKind::Class,
            }) if target.literal() == Some("::C")
        )));

        let effects = invocation.effect_footprint();
        assert!(
            !effects
                .accesses()
                .iter()
                .any(|access| access.domain == WorldStateDomain::InterpreterPolicy)
        );
        assert!(effects.callback().kinds.contains(CallbackKinds::SCRIPT));
    }

    #[test]
    fn create_with_namespace_retains_explicit_private_identity() {
        let registry = CommandRegistry::build_default();
        let transitions = registry
            .resolve_invocation(
                "oo::class",
                &["createWithNamespace", "::C", "::private::C", "{}"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("createWithNamespace must resolve")
            .state_transitions();
        assert!(transitions.facts().iter().any(|fact| matches!(
            &fact.transition,
            StateTransition::ObjectDispatch(ObjectDispatchTransition::Create {
                private_namespace: ObjectPrivateNamespace::Named(namespace),
                ..
            }) if namespace.literal() == Some("::private::C")
        )));
    }

    #[test]
    fn every_registered_metaclass_create_uses_shared_lifecycle_data() {
        let registry = CommandRegistry::build_default();
        for (metaclass, dialect) in [
            ("oo::class", Some(SurfaceQuery::core(Family::Tcl, "8.6"))),
            ("oo::abstract", Some(SurfaceQuery::core(Family::Tcl, "9.0"))),
            (
                "oo::configurable",
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            ),
            (
                "oo::singleton",
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            ),
        ] {
            let transitions = registry
                .resolve_invocation(metaclass, &["create", "::C", "{}"], dialect)
                .unwrap_or_else(|| panic!("{metaclass} create must resolve"))
                .state_transitions();
            assert!(
                transitions.facts().iter().any(|fact| {
                    fact.commit == StateTransitionCommit::OnOkOnly
                        && matches!(
                            &fact.transition,
                            StateTransition::CommandBinding(CommandBindingTransition::Define {
                                name,
                                ..
                            }) if name.literal() == Some("::C")
                        )
                }),
                "{metaclass} lacked its named command-binding lifecycle"
            );
            assert!(
                transitions.facts().iter().any(|fact| {
                    fact.commit == StateTransitionCommit::OnOkOnly
                        && matches!(
                            &fact.transition,
                            StateTransition::ObjectDispatch(ObjectDispatchTransition::Create {
                                target: ObjectDispatchTarget::Named(target),
                                kind: ObjectDispatchKind::Class,
                                ..
                            }) if target.literal() == Some("::C")
                        )
                }),
                "{metaclass} lacked its class dispatch lifecycle"
            );
        }
    }
}
