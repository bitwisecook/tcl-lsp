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

//! `oo::object` — the root class of the `TclOO` object hierarchy.
use crate::prelude::*;

const OBJECT_CREATE_TRANSITION_DOMAINS: &[StateTransitionDomain] = &[
    StateTransitionDomain::CommandBindings,
    StateTransitionDomain::Namespaces,
    StateTransitionDomain::ObjectDispatch,
];

const OBJECT_INTERP_EFFECT_COVERAGE: &[TransitionEffectCoverage] = &[TransitionEffectCoverage {
    source: WorldEffectWriteSource::LegacySideEffect(SideEffectTarget::InterpState),
    domains: &[WorldStateDomain::InterpreterPolicy],
}];

const OBJECT_CREATE_EFFECTS: WorldEffectDescriptor = WorldEffectDescriptor {
    composition: WorldEffectComposition::Extend,
    static_footprint: StaticEffectFootprint {
        accesses: &[],
        // Constructors are ordinary TclOO method callbacks. The new object's
        // state is live during the callback, but constructor failure rolls it
        // back before the factory's abrupt completion escapes.
        callback: CallbackEffect {
            kinds: CallbackKinds::SCRIPT,
            reentrancy: Reentrancy::CurrentInterpreter,
        },
    },
    resolver: None,
    dynamic_fallback: WorldEffectDynamicFallback::ConservativeUnknownInvocation,
};

const OBJECT_CREATE_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(object_create_state_transitions),
    argument_shape: StateTransitionArgumentShape::Positional,
    dynamic_widening: &[StateTransitionWideningRule {
        operands: StateTransitionOperandLayout::Indices(&[1]),
        domains: OBJECT_CREATE_TRANSITION_DOMAINS,
    }],
    effect_coverage: OBJECT_INTERP_EFFECT_COVERAGE,
    commit: StateTransitionCommit::OnOkOnly,
};

const OBJECT_NEW_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(object_new_state_transitions),
    argument_shape: StateTransitionArgumentShape::Independent,
    dynamic_widening: &[],
    effect_coverage: OBJECT_INTERP_EFFECT_COVERAGE,
    commit: StateTransitionCommit::OnOkOnly,
};

fn object_create_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    let Some(target) = TransitionSubject::from_argument(arguments, 1) else {
        return transitions;
    };
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
            private_namespace: ObjectPrivateNamespace::Fresh,
            kind: ObjectDispatchKind::Object,
        },
    ));
    transitions
}

fn object_new_state_transitions(_arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    transitions.push(StateTransition::ObjectDispatch(
        ObjectDispatchTransition::Create {
            target: ObjectDispatchTarget::Fresh,
            private_namespace: ObjectPrivateNamespace::Fresh,
            kind: ObjectDispatchKind::Object,
        },
    ));
    transitions
}

// `oo::object create` / `new` mint a new command and its instance
// namespace (interpreter command-table state); `destroy` removes one the
// same way. No file I/O, process, or channel involvement, so a single
// `InterpState` write covers the whole exported surface — matching every
// other `TclOO` metaclass spec (`oo::class`, `oo::abstract`,
// `oo::configurable`, `oo::singleton`, `oo::copy`).
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "oo::object method ?arg ...?",
    ..FormSpec::DEFAULT
}];

/// The universal object-method surface every `TclOO` object (class or
/// instance) exposes, restricted to the methods reachable *directly*
/// through an object's public name: object.n's `destroy` (its only
/// exported method) plus class.n's `create` / `new` (exported on any
/// class — including `oo::object` itself, which, like every class, is
/// also an instance of `oo::class`, so it answers to `create`/`new`
/// too). object.n additionally documents `eval` / `unknown` /
/// `variable` / `varname` / `<cloned>`, and class.n documents the
/// non-exported `createWithNamespace`, but none of those six are
/// modelled here: they are all **non-exported**, so calling them
/// through an object's public name fails with "unknown method" —
/// confirmed by object.n's own EXAMPLES section, where `$obj variable
/// count` errors exactly that way even though `variable` is a real,
/// documented method. They're reachable only via `my` from inside a
/// method body, or after explicitly re-exporting them with
/// `oo::define`/`oo::objdefine`. `destroy` is also the
/// registry-declared destructive method the command-binding lattice
/// consults: `NAME destroy` deletes the `NAME` command, so later
/// dispatches fall through to `unknown`.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "destroy",
        arity: Arity::exact(0),
        detail: "Destroy obj, invoking any destructors defined on its class in the process, and delete its command (equivalent to renaming the command out of existence). If obj is itself a class, all of its subclasses, instances, and any objects it has been mixed into are destroyed too. Always returns the empty string.",
        synopsis: "obj destroy",
        destructive: true,
        // Documented as equivalent to `rename obj {}` (object.n), the
        // flagship example for this trait — the same fire-and-forget
        // teardown idiom as `close` / `unset` / `rename foo {}`.
        traits: Traits::FIRE_AND_FORGET_TEARDOWN,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "new",
        arity: Arity::any(),
        detail: "Create a new instance with an automatically generated unique name, passing arg ... to the constructor (whose own return value is ignored). Returns the fully qualified name of the new object. If the constructor raises an error or other non-OK result, the half-built object is destroyed and the error propagates from this call.",
        synopsis: "class new ?arg ...?",
        return_type: Some(TclType::String),
        world_effects: Some(OBJECT_CREATE_EFFECTS),
        state_transitions: Some(OBJECT_NEW_TRANSITIONS),
        // Unlike `oo::class` (which hides its own `new`, per class.n:
        // "the oo::class object hides the new method on itself"),
        // `oo::object` does not hide its own `new` — object.n's
        // DESCRIPTION ("Instances of objects may be made with either
        // the create or new methods of the oo::object object itself")
        // and its EXAMPLES (`set obj [oo::object new]`) both show it
        // succeeding directly.
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "create",
        arity: Arity::at_least(1),
        detail: "Create a new instance named name (resolved in the calling namespace if not already fully qualified), passing arg ... to the constructor (whose own return value is ignored). Returns the fully qualified name of the new object. If the constructor raises an error or other non-OK result, the half-built object is destroyed and the error propagates from this call.",
        synopsis: "class create name ?arg ...?",
        // `oo::object create name` binds `name` as the new object's
        // command (e.g. `oo::object create Foo; Foo destroy`). Index 0
        // is the word right after `create`; `oo::object` has no options
        // of its own to skip first — the same shape as `interp
        // create`'s subcommand spec.
        defines_command_at: Some(0),
        return_type: Some(TclType::String),
        world_effects: Some(OBJECT_CREATE_EFFECTS),
        state_transitions: Some(OBJECT_CREATE_TRANSITIONS),
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::object",
        // `NOT_PROC_FACTORY`: `oo::object create Name { … }` is a
        // four-token `HEAD NAME BRACED BRACED` call — the same shape
        // `oo::class` / `oo::abstract` / `oo::configurable` match, so it
        // needs the same guard against the proc-body factory-wrapper
        // rescan (`scan_factory_candidates`) misfiling it as a
        // tcllib-style factory-wrapper candidate (see the rationale on
        // `oo_abstract.rs`). `LANGUAGE_KEYWORD`: coloured as a keyword
        // for semantic tokens like every other `oo::*` metaclass command
        // (`oo::class`, `oo::define`, `oo::objdefine`, `oo::abstract`,
        // `oo::configurable`, `oo::singleton`). Deliberately *not*
        // `DEFINES_PROCEDURE`: unlike those metaclasses, `oo::object`
        // has no `definition_body` grammar, so `dispatch_definer`
        // (signature_scan/walker.rs) would fall through to its
        // `DEFINES_PROCEDURE` arm and misparse `oo::object create Name
        // {ctorArg}` as a `proc`-shaped definition (`create` as the proc
        // name, `Name` as its arg list) — `IS_OO_METACLASS` plus the
        // absence of a definition body is what correctly keeps `create`
        // classified as an instance factory, not a class definer (see
        // the `oo_object_instance_creation_not_a_class_definer` test).
        traits: Traits::IS_OO_METACLASS
            | Traits::OBJECT_COMMAND_SURFACE
            | Traits::LANGUAGE_KEYWORD
            | Traits::NOT_PROC_FACTORY,
        // Verified absent (soft-404 "URL Not Found" page, HTTP 200) from
        // the tcl-lang.org manpage tree for 8.4 and 8.5; present with
        // byte-identical NAME/SYNOPSIS/CLASS HIERARCHY/DESCRIPTION/
        // CONSTRUCTOR/DESTRUCTOR/EXPORTED METHODS/NON-EXPORTED METHODS/
        // EXAMPLES text across 8.6.18, 9.0.4, and 9.1b0 (only the
        // version banner and the package name in SYNOPSIS differ:
        // `package require TclOO` in 8.6 vs `package require tcl::oo`
        // from 9.0 on) — `TclOO` shipped built into the Tcl core in
        // 8.6 and object.n is unchanged since.
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        subcommands: SUBCOMMANDS,
        // Every user method also dispatches through an object command, so
        // unknown words must stay valid.
        allow_unknown_subcommands: true,
        hover: Some(HoverSnippet {
            summary: "root class of the class hierarchy",
            synopsis: &[
                "oo::object method ?arg ...?",
                "oo::object destroy",
                "oo::object new ?arg ...?",
                "oo::object create name ?arg ...?",
            ],
            snippet: "The oo::object class is the root of the TclOO class hierarchy: every object, including every class (since classes are themselves objects), is ultimately an instance of oo::object. Objects are identified by their command name and keep their identity across `rename`. New instances are made with the `create` or `new` methods of oo::object itself, or of any of its subclasses (see `oo::class`); per-object configuration (added methods, mixins, filters, and so on) is done with `oo::objdefine`. Each object has its own instance namespace holding its instance variables; that namespace becomes the current namespace for the duration of any method call on the object, and is deleted when the object is destroyed. It also holds the object's `my` command, used from inside the object's own methods to reach its non-exported methods (`eval`, `unknown`, `variable`, `varname`, `<cloned>`); unlike `destroy`, none of those five are reachable through the object's public name. oo::object defines neither an explicit constructor nor an explicit destructor. TclOO ships built into the core interpreter: `package require TclOO` in Tcl 8.6, `package require tcl::oo` from Tcl 9.0 on.",
            source: "Tcl man page object.n, class.n",
            examples: "set obj [oo::object new]\noo::objdefine $obj method greet {} {\n    my variable count\n    puts \"hello[incr count]\"\n}\n$obj greet\n$obj greet\n$obj destroy",
            return_value: "The fully qualified name of the new object for new and create; the empty string for destroy; otherwise whatever the invoked method returns.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use crate::dialects::DialectSet;
    use crate::{
        CallbackKinds, CommandBindingTransition, CommandRegistry, ObjectDispatchKind,
        ObjectDispatchTarget, ObjectDispatchTransition, Reentrancy, StateTransition,
        StateTransitionCommit, StateTransitionKnowledge, WorldStateDomain,
    };

    #[test]
    fn object_factories_keep_named_and_fresh_lifecycle_distinct() {
        let registry = CommandRegistry::build_default();
        let named = registry
            .resolve_invocation("oo::object", &["create", "::obj"], DialectSet::TCL86)
            .expect("oo::object create must resolve");
        let named_transitions = named.state_transitions();
        assert!(
            named_transitions
                .facts()
                .iter()
                .all(|fact| { fact.commit == StateTransitionCommit::OnOkOnly })
        );
        assert!(named_transitions.facts().iter().any(|fact| matches!(
            &fact.transition,
            StateTransition::CommandBinding(CommandBindingTransition::Define { name, .. })
                if name.literal() == Some("::obj")
        )));
        assert!(named_transitions.facts().iter().any(|fact| matches!(
            &fact.transition,
            StateTransition::ObjectDispatch(ObjectDispatchTransition::Create {
                target: ObjectDispatchTarget::Named(target),
                kind: ObjectDispatchKind::Object,
                ..
            }) if target.literal() == Some("::obj")
        )));

        let fresh = registry
            .resolve_invocation("oo::object", &["new"], DialectSet::TCL86)
            .expect("oo::object new must resolve");
        assert!(
            fresh
                .state_transitions()
                .facts()
                .iter()
                .any(|fact| matches!(
                    fact.transition,
                    StateTransition::ObjectDispatch(ObjectDispatchTransition::Create {
                        target: ObjectDispatchTarget::Fresh,
                        ..
                    })
                ))
        );

        for invocation in [named, fresh] {
            let effects = invocation.effect_footprint();
            assert!(
                !effects
                    .accesses()
                    .iter()
                    .any(|access| access.domain == WorldStateDomain::InterpreterPolicy)
            );
            assert!(effects.callback().kinds.contains(CallbackKinds::SCRIPT));
            assert_eq!(
                effects.callback().reentrancy,
                Reentrancy::CurrentInterpreter
            );
        }
    }

    #[test]
    fn destroy_is_not_misstamped_as_a_factory_lifecycle() {
        let registry = CommandRegistry::build_default();
        let facts = registry
            .resolve_invocation("oo::object", &["destroy"], DialectSet::TCL86)
            .expect("oo::object destroy must resolve")
            .facts();
        assert!(matches!(
            facts.state_transitions,
            StateTransitionKnowledge::UnknownInvocation
        ));
    }
}
