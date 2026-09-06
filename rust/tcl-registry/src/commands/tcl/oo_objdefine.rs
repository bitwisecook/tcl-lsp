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

//! `oo::objdefine` — define per-object members.
use crate::prelude::*;

const OO_OBJDEFINE_TRANSITION_DOMAINS: &[StateTransitionDomain] =
    &[StateTransitionDomain::ObjectDispatch];

const OO_OBJDEFINE_EFFECT_COVERAGE: &[TransitionEffectCoverage] = &[
    TransitionEffectCoverage {
        source: WorldEffectWriteSource::DeclaredWorldEffect,
        domains: &[WorldStateDomain::OoDispatch],
    },
    TransitionEffectCoverage {
        source: WorldEffectWriteSource::LegacySideEffect(SideEffectTarget::InterpState),
        domains: &[WorldStateDomain::InterpreterPolicy],
    },
];

const OO_OBJDEFINE_EFFECTS: WorldEffectDescriptor = WorldEffectDescriptor {
    composition: WorldEffectComposition::Extend,
    static_footprint: StaticEffectFootprint {
        accesses: &[StaticEffectAccess::new(
            WorldStateDomain::OoDispatch,
            EffectAccessMode::ReadWrite,
            StaticInterpreterScope::Current,
            StaticNamespaceScope::Current,
            StaticSubjectScope::Wildcard,
        )],
        callback: CallbackEffect {
            kinds: CallbackKinds::SCRIPT,
            reentrancy: Reentrancy::CurrentInterpreter,
        },
    },
    resolver: None,
    dynamic_fallback: WorldEffectDynamicFallback::ConservativeUnknownInvocation,
};

const OO_OBJDEFINE_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(oo_objdefine_state_transitions),
    argument_shape: StateTransitionArgumentShape::Independent,
    dynamic_widening: &[StateTransitionWideningRule {
        operands: StateTransitionOperandLayout::Indices(&[0]),
        domains: OO_OBJDEFINE_TRANSITION_DOMAINS,
    }],
    effect_coverage: OO_OBJDEFINE_EFFECT_COVERAGE,
    commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
};

fn oo_objdefine_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    if let Some(target) = TransitionSubject::from_argument(arguments, 0) {
        transitions.push(StateTransition::ObjectDispatch(
            ObjectDispatchTransition::Configure {
                target,
                layer: ObjectDispatchLayer::Object,
            },
        ));
    }
    transitions
}

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[
    FormSpec {
        synopsis: "oo::objdefine object defScript",
        ..FormSpec::DEFAULT
    },
    // Present alongside the script form in every version that has
    // `oo::objdefine` at all (8.6, 9.0, 9.1) — the two-form SYNOPSIS is
    // identical across all three fetched manpages.
    FormSpec {
        synopsis: "oo::objdefine object subcommand arg ?arg ...?",
        ..FormSpec::DEFAULT
    },
];

/// Completion / hover metadata for the subcommand word at position 1
/// (`oo::objdefine object SUBCOMMAND ...`) — every member word
/// `oo::objdefine` itself recognises. This is a strict subset of
/// `oo::define`'s vocabulary: objects have no `constructor` /
/// `destructor` / `classmethod` / `superclass` / `definitionnamespace`
/// (those only make sense for a class). Built from the 8.6 / 9.0 / 9.1
/// `TclCmd/define` manpages (8.4 and 8.5 have no such page at all — `TclOO`
/// is 8.6+; the 9.0 and 9.1 pages are byte-for-byte identical for this
/// command, so nothing here is 9.1-specific). Position 1 can *also*
/// legitimately be the entire bare `defScript` script-form body instead
/// of one of these words, so `oo::objdefine` has no `closed_value_args`
/// entry for this position — this is completion/hover data only,
/// mirroring `oo::define`'s `OO_DEFINE_SUBCOMMAND_VALUES` and `open`'s
/// `ACCESS_VALUES`.
const OO_OBJDEFINE_SUBCOMMAND_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "class",
        detail: "Changes the object's class after creation; the new class's constructors are not run, so the object may be left in an inconsistent state until reconfigured.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "deletemethod",
        detail: "Removes the named methods from the object itself (e.g. ones added via method), without affecting the classes it is an instance of.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "export",
        detail: "Makes the named methods, however they were defined, callable from outside the object through its own command; overrides class-level visibility for that name.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "filter",
        detail: "Adds, by default, per-object method-call filters that guard calls to this object; combines with any filters set by the classes it is an instance of.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "forward",
        detail: "Defines a forwarded method: calling name invokes cmdName, resolved by the object's own namespace rules, with any given arg words prepended ahead of the caller's own arguments. Delete a forwarded method with the method subcommand.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "method",
        detail: "Creates or replaces an object method; runs in the object's own namespace. An empty bodyScript makes it a no-op, it does not remove it — use deletemethod for that. Exported by default when name starts with a lowercase letter; Tcl 9.0+ also accepts an explicit -export/-private/-unexport option before argList to override that default.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "mixin",
        detail: "Sets, by default replacing, the classes mixed into this one object, ahead of its own classes' hierarchy in method resolution.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "private",
        detail: "Runs a nested forward/method/variable definition — or, as `private {script}`, an entire nested script — in a private definition context, restricting the members it defines to internal use.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "renamemethod",
        detail: "Renames an existing object method from fromName to toName, preserving its export status; toName must not already name a method on the object.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "self",
        detail: "Bare form only — takes no subcommand or script (unlike oo::define's self): returns the name of the object currently being configured.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "unexport",
        detail: "Makes the named methods callable only via my from inside the object's own methods, not from outside; overrides class-level visibility for that name.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "variable",
        detail: "Adds, by default, the named variables to the set automatically linked into every method of the object, equivalent to running my variable on each name first thing in the body; names must be simple, with no :: or array-element syntax.",
        ..ArgValue::DEFAULT
    },
];

use super::oo_define::oo_define_arg_roles;
use tcl_dialect::model::SpecSurface;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::objdefine",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::LANGUAGE_KEYWORD
            | Traits::INSTALLS_NAMED_DEFINITION
            | Traits::NEVER_INLINE_BODY,
        surface: Some(SpecSurface::TCL86_PLUS),
        arity: Arity::at_least(2),
        // `oo::objdefine` has the same body-shape rules as
        // `oo::define`; share the resolver.
        arg_role_resolver: Some(oo_define_arg_roles),
        arg_role_resolver_roles: &[ArgRole::Body],
        return_type: Some(TclType::String),
        // Same structural-body rule as `oo::define` — bodies
        // run in a per-object definition context, not the caller's
        // frame.
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "Reconfigure an existing object (or a class acting as an instance object): per-object methods, mixins, variables, and more.",
            synopsis: &[
                "oo::objdefine object defScript",
                "oo::objdefine object subcommand arg ?arg ...?",
            ],
            snippet: "oo::objdefine object defScript evaluates defScript as a nested definition script whose member commands (method, variable, mixin, and so on) reconfigure object directly, while oo::objdefine object subcommand arg ?arg ...? runs exactly one definition subcommand. Unlike oo::define, changes never propagate to other instances of object's class — only to object itself (and, when object is a class, to that class's own per-instance behaviour as an object, distinct from its behaviour as a class). Slot-taking subcommands (variable, mixin, filter) accept an optional leading operation — -append/-set/-clear, plus -appendifnew/-prepend/-remove from Tcl 9.0 — before their name list; omitting it uses each subcommand's own default (append for variable/filter, replace for mixin). Tcl 9.0 also added the private and self (bare, returns the name of the object currently being configured) subcommands, plus an optional -export/-private/-unexport option before method's argList.",
            source: "Tcl oo::objdefine(n)",
            examples: "oo::class create Greeter\nGreeter create g\noo::define Greeter method hello {} {\n    puts \"hello\"\n}\noo::objdefine g {\n    method greet {} {\n        my Announce \"world \"\n        my hello\n    }\n    forward Announce ::puts -nonewline\n    unexport hello\n}\ng greet",
            return_value: "The empty string for most definition subcommands; the script form's own result is whatever its last embedded command returns.",
        }),
        forms: FORMS,
        arg_values: &[(1, OO_OBJDEFINE_SUBCOMMAND_VALUES)],
        side_effects: SIDE_EFFECTS,
        world_effects: Some(OO_OBJDEFINE_EFFECTS),
        state_transitions: Some(OO_OBJDEFINE_TRANSITIONS),
        definition_body: Some(&crate::definer::TCLOO_GRAMMAR),
        analyser_hook: Some(crate::hooks::AnalyserHookId::OoObjdefine),
        ..CommandSpec::DEFAULT
    }
}
