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

//! `oo::define` — define class members.
use crate::prelude::*;

const OO_DEFINE_TRANSITION_DOMAINS: &[StateTransitionDomain] =
    &[StateTransitionDomain::ObjectDispatch];

const OO_DEFINE_EFFECT_COVERAGE: &[TransitionEffectCoverage] = &[
    TransitionEffectCoverage {
        source: WorldEffectWriteSource::DeclaredWorldEffect,
        domains: &[WorldStateDomain::OoDispatch],
    },
    TransitionEffectCoverage {
        source: WorldEffectWriteSource::LegacySideEffect(SideEffectTarget::InterpState),
        domains: &[WorldStateDomain::InterpreterPolicy],
    },
];

const OO_DEFINE_EFFECTS: WorldEffectDescriptor = WorldEffectDescriptor {
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

const OO_DEFINE_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(oo_define_state_transitions),
    argument_shape: StateTransitionArgumentShape::Independent,
    dynamic_widening: &[StateTransitionWideningRule {
        operands: StateTransitionOperandLayout::Indices(&[0]),
        domains: OO_DEFINE_TRANSITION_DOMAINS,
    }],
    effect_coverage: OO_DEFINE_EFFECT_COVERAGE,
    // Definition scripts run sequentially. An earlier member mutation remains
    // visible when a later definition command raises an abrupt completion.
    commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
};

fn oo_define_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    if let Some(target) = TransitionSubject::from_argument(arguments, 0) {
        transitions.push(StateTransition::ObjectDispatch(
            ObjectDispatchTransition::Configure {
                target,
                layer: ObjectDispatchLayer::Class,
            },
        ));
    }
    transitions
}

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[
    FormSpec {
        kind: FormKind::Default,
        synopsis: "oo::define class defScript",
        dialects: None,
    },
    // Present alongside the script form in every version that has
    // `oo::define` at all (8.6, 9.0, 9.1) — the two-form SYNOPSIS is
    // identical across all three fetched manpages.
    FormSpec {
        kind: FormKind::Default,
        synopsis: "oo::define class subcommand arg ?arg ...?",
        dialects: None,
    },
];

/// Subcommands recognised by ``oo::define`` / ``oo::objdefine``.
/// Used to disambiguate the script-form (`oo::define Target {body}`)
/// from a subcommand call where `args[1]` is one of these words.
///
/// `classmethod` / `initialise` / `initialize` / `private` /
/// `definitionnamespace` are Tcl 9.0+ only (absent from the 8.6
/// `TclCmd/define.htm` SYNOPSIS/subcommand list, present in both the
/// 9.0 and 9.1 `TclCmd/define.html` pages); every other entry is
/// documented already at 8.6.
///
/// This list is deliberately **dialect-blind**: it answers "is word 1 a
/// member keyword rather than the bare `defScript`?", a purely structural
/// question that must resolve the same way under every dialect, so an
/// `oo::define Cls classmethod m {} {…}` written under 8.6 still parses as
/// the subcommand form (and its body still folds and highlights) instead of
/// being mistaken for a script.  *Availability* is the separate fact, carried
/// by the matching [`crate::definer::MemberSpec::dialects`] in the `TclOO`
/// grammar and reported by the analyser as W002.
///
/// `property` is not actually a plain
/// `oo::define`/`oo::class` subcommand — it is installed by
/// `oo::configurable` (Tcl 9.0+) — but is kept here since a real,
/// legal `oo::define SomeConfigurableClass property ...` call must
/// still resolve its trailing body correctly. Mirrored, with a
/// per-entry description and `min_tcl`, by
/// [`OO_DEFINE_SUBCOMMAND_VALUES`] below.
const OO_DEFINE_SUBCOMMANDS: &[&str] = &[
    "constructor",
    "destructor",
    "method",
    "classmethod",
    "initialise",
    "initialize",
    "private",
    "self",
    "property",
    "filter",
    "export",
    "unexport",
    "deletemethod",
    "renamemethod",
    "forward",
    "mixin",
    "superclass",
    "variable",
    "definitionnamespace",
];

/// Completion / hover metadata for the subcommand word at position 1
/// (`oo::define class SUBCOMMAND ...`) — every member word
/// `oo::define` recognises, each with a `min_tcl` reflecting the
/// version table built from the 8.6 / 9.0 / 9.1 `TclCmd/define`
/// manpages (8.4 and 8.5 have no such page at all — `TclOO` is 8.6+).
/// Position 1 can *also* legitimately be the entire bare `defScript`
/// script-form body instead of one of these words (see
/// [`OO_DEFINE_SUBCOMMANDS`] above), so `oo::define` has no
/// `closed_value_args` entry for this position — this is
/// completion/hover data only, mirroring `open`'s `ACCESS_VALUES`.
const OO_DEFINE_SUBCOMMAND_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "classmethod",
        detail: "Defines a class method, or promotes an existing method to one: callable on the class itself as well as on its instances, with the invoking object set to the class.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "constructor",
        detail: "Defines or replaces the class constructor: argList is a proc-style parameter list, bodyScript runs in the new object's own namespace, and an empty bodyScript deletes the constructor. Use next inside it to invoke superclass constructors.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "definitionnamespace",
        detail: "Sets the namespace consulted for definition-command lookups (?-class?, the default, or -instance) to namespaceName, letting a custom metaclass add its own definition-body vocabulary.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "deletemethod",
        detail: "Removes the named methods from the class without affecting its superclasses, subclasses, or existing instances beyond their method-resolution call chains.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "destructor",
        detail: "Defines or replaces the class destructor (it takes no arguments); runs in the object's own namespace, and an empty bodyScript deletes it. Errors raised while destroying an object go to the bgerror handler, not back to the caller.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "export",
        detail: "Makes the named methods, however they were defined, callable from outside an instance through the instance command; a subclass's export overrides its superclass's for that name.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "filter",
        detail: "Adds, by default, method-call filters that guard every call to the class's methods; naming a not-yet-defined method is fine, since a subclass may still supply it.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "forward",
        detail: "Defines a forwarded method: calling name runs cmdName, resolved by the invoking object's own namespace rules, with any given arg words prepended ahead of the caller's own arguments.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "initialise",
        detail: "Evaluates script immediately, in the class object's own namespace — the idiomatic place to set up classvariable-backed class-scoped state.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "initialize",
        detail: "American-spelling alias for initialise.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "method",
        detail: "Defines or replaces a method; runs in the object's own namespace. An empty bodyScript makes it a no-op, it does not remove it — use deletemethod for that. Exported by default when name starts with a lowercase letter; Tcl 9.0+ also accepts an explicit -export/-private/-unexport option before argList to override that default.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "mixin",
        detail: "Sets, by default replacing, the classes mixed into every instance of this class, ahead of the class's own hierarchy in method resolution. Takes an optional leading slot operation before the class list.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "private",
        detail: "Runs a nested forward, method, self, or variable definition — the only class definition commands private affects — or, as `private {script}`, an entire nested script, in a private definition context restricting those members to internal use. Every other definition command (constructor, destructor, classmethod, and so on) behaves identically inside or outside private.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "property",
        detail: "Declares a configurable property (name ?-get script? ?-set script? ?-kind readable|writable|readwrite?); only resolves on a class created via oo::configurable (or a subclass of it), which supplies this definition command.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "renamemethod",
        detail: "Renames an existing class method from fromName to toName, preserving its export status; toName must not already name a method on the class.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "self",
        detail: "Equivalent to invoking oo::objdefine on the class itself: `self subcommand ...` behaves like `oo::objdefine class subcommand ...`, and `self {script}` runs script the same way. Tcl 9.0+ also documents self with no further words at all, returning the name of the class currently being configured; Tcl 8.6 requires at least one further word and raises a wrong # args error on a bare self.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "superclass",
        detail: "Sets, by default replacing, the class's direct superclasses; an empty list is equivalent to oo::object, and the superclasses of oo::object and oo::class themselves cannot be changed.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "unexport",
        detail: "Makes the named methods callable only via my from inside the object's own methods, not from outside; a subclass's unexport overrides its superclass's for that name.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "variable",
        detail: "Adds, by default, the named variables to the set automatically linked into every method/constructor/destructor of the class, equivalent to running my variable on each name first thing in the body; names must be simple, with no :: or array-element syntax.",
        ..ArgValue::DEFAULT
    },
];

/// Resolve body argument indices for `oo::define` / `oo::objdefine`.
///
/// * `oo::define Target body` (script form, when `args[1]` is not a
///   recognised subcommand) → body at index 1.
/// * `oo::define Target constructor args body` → body at index 3.
/// * `oo::define Target destructor body` → body at index 2.
/// * `oo::define Target method name args body` → body at last index.
/// * `oo::define Target initialise body` / `initialize body` /
///   `private body` → body at index 2.
/// * `oo::define Target self constructor args body` → body at index 4.
/// * `oo::define Target self destructor body` → body at index 3.
/// * `oo::define Target self method name args body` → body at last
///   index.
/// * `oo::define Target property -set BODY ?-get BODY?` →
///   bodies after each `-set` / `-get` flag.
pub(crate) fn oo_define_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let n = args.len();
    if n == 2 && !OO_DEFINE_SUBCOMMANDS.contains(&args[1]) {
        return vec![(1, ArgRole::Body)];
    }
    if n < 2 {
        return Vec::new();
    }
    let Ok(last) = u8::try_from(n - 1) else {
        return Vec::new();
    };
    match args[1] {
        "constructor" if n >= 4 => vec![(3, ArgRole::Body)],
        "method" | "classmethod" if n >= 5 => vec![(last, ArgRole::Body)],
        // `destructor`/`initialise`/`initialize`/`private` all take a
        // body at index 2.
        "destructor" | "initialise" | "initialize" | "private" if n >= 3 => {
            vec![(2, ArgRole::Body)]
        }
        "self" if n >= 3 => match args[2] {
            "constructor" if n >= 5 => vec![(4, ArgRole::Body)],
            "destructor" if n >= 4 => vec![(3, ArgRole::Body)],
            "method" | "classmethod" if n >= 6 => vec![(last, ArgRole::Body)],
            _ => Vec::new(),
        },
        "property" => collect_property_body_roles(args, 2),
        _ => Vec::new(),
    }
}

/// `oo::define Target property name ?-set BODY? ?-get BODY?` →
/// flag-keyed bodies. `start` is the index of the first option flag
/// (2 for `oo::define Target property`, 0 for inner `property` —
/// which folding handles separately).
pub(crate) fn collect_property_body_roles(args: &[&str], start: usize) -> Vec<(u8, ArgRole)> {
    let n = args.len();
    if n == 0 {
        return Vec::new();
    }
    args.iter()
        .enumerate()
        .skip(start)
        .take(n.saturating_sub(start + 1))
        .filter_map(|(i, &a)| {
            if (a == "-set" || a == "-get") && i + 1 < n {
                u8::try_from(i + 1).ok().map(|idx| (idx, ArgRole::Body))
            } else {
                None
            }
        })
        .collect()
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::define",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::LANGUAGE_KEYWORD
            | Traits::INSTALLS_NAMED_DEFINITION
            | Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(2),
        arg_roles: &[(0, ArgRole::Name)],
        arg_role_resolver: Some(oo_define_arg_roles),
        return_type: Some(TclType::String),
        // Every body argument that `oo_define_arg_roles`
        // surfaces is a TclOO definition / dispatch body, never a
        // caller-frame body.  Stamping `Structural` here covers all
        // the script-bearing forms (constructor / destructor /
        // method / classmethod / initialise / initialize / private /
        // self.* / property -set / -get) plus the bare-script form
        // `oo::define Cls {body}`.
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "Reconfigure an existing class: methods, constructor/destructor, superclasses, mixins, and more.",
            synopsis: &[
                "oo::define class defScript",
                "oo::define class subcommand arg ?arg ...?",
            ],
            snippet: "oo::define class defScript evaluates defScript as a nested definition script whose member commands (method, constructor, variable, and so on) apply to class, while oo::define class subcommand arg ?arg ...? runs exactly one definition subcommand directly. Changes propagate to every subclass and instance of class; oo::objdefine is the equivalent command for reconfiguring a single object (including a class acting as an instance object). Slot-taking subcommands (variable, superclass, mixin, filter) accept an optional leading operation — -append/-set/-clear, plus -appendifnew/-prepend/-remove from Tcl 9.0 — before their name list; omitting it uses each subcommand's own default (append for variable/filter, replace for superclass/mixin). Tcl 9.0 also added the classmethod, private, initialise/initialize, and definitionnamespace subcommands (none exist under Tcl 8.6); a class created via the Tcl-9.0+ oo::configurable metaclass additionally accepts a property subcommand for declaring configurable properties.",
            source: "Tcl oo::define(n)",
            examples: "oo::class create Greeter\noo::define Greeter {\n    variable greeting\n    constructor {msg} {\n        set greeting $msg\n    }\n    method greet {name} {\n        puts \"$greeting, $name!\"\n    }\n    export greet\n}\n[Greeter new \"Hello\"] greet World",
            return_value: "The empty string for most definition subcommands; the script form's own result is whatever its last embedded command returns.",
        }),
        forms: FORMS,
        arg_values: &[(1, OO_DEFINE_SUBCOMMAND_VALUES)],
        side_effects: SIDE_EFFECTS,
        world_effects: Some(OO_DEFINE_EFFECTS),
        state_transitions: Some(OO_DEFINE_TRANSITIONS),
        definition_body: Some(&crate::definer::TCLOO_GRAMMAR),
        analyser_hook: Some(crate::hooks::AnalyserHookId::OoDefine),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::oo_define_arg_roles;
    use crate::arg_role::ArgRole;
    use crate::dialects::DialectSet;
    use crate::{
        CallbackKinds, CommandRegistry, ObjectDispatchLayer, ObjectDispatchTransition, Reentrancy,
        StateTransition, StateTransitionCommit, WorldStateDomain,
    };

    // POSITIVE: every subcommand merged into the shared `n >= 3 => index 2`
    // arm (`destructor` / `initialise` / `initialize` / `private`) must
    // still resolve a body argument at index 2.
    #[test]
    fn index2_body_subcommands_resolve() {
        for sub in ["destructor", "initialise", "initialize", "private"] {
            let roles = oo_define_arg_roles(&["Target", sub, "{ body }"]);
            assert_eq!(
                roles,
                vec![(2, ArgRole::Body)],
                "{sub}: expected body at index 2"
            );
        }
    }

    // POSITIVE: arms NOT merged keep their distinct indices.
    #[test]
    fn distinct_arms_keep_their_indices() {
        assert_eq!(
            oo_define_arg_roles(&["Target", "constructor", "args", "{ body }"]),
            vec![(3, ArgRole::Body)]
        );
        assert_eq!(
            oo_define_arg_roles(&["Target", "method", "name", "args", "{ body }"]),
            vec![(4, ArgRole::Body)]
        );
        // `self destructor` is a separate (inner) arm → index 3, not 2.
        assert_eq!(
            oo_define_arg_roles(&["Target", "self", "destructor", "{ body }"]),
            vec![(3, ArgRole::Body)]
        );
    }

    #[test]
    fn class_and_object_configuration_keep_distinct_registry_layers() {
        let registry = CommandRegistry::build_default();
        for (command, expected_layer) in [
            ("oo::define", ObjectDispatchLayer::Class),
            ("oo::objdefine", ObjectDispatchLayer::Object),
        ] {
            let invocation = registry
                .resolve_invocation(
                    command,
                    &["::Target", "method", "m", "{}", "{}"],
                    DialectSet::TCL86,
                )
                .expect("TclOO definition command must resolve");
            let transitions = invocation.state_transitions();
            assert!(transitions.facts().iter().any(|fact| {
                fact.commit == StateTransitionCommit::MayCommitBeforeAbruptCompletion
                    && matches!(
                        &fact.transition,
                        StateTransition::ObjectDispatch(ObjectDispatchTransition::Configure {
                            target,
                            layer,
                        }) if target.literal() == Some("::Target") && *layer == expected_layer
                    )
            }));

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

    // NEGATIVE: the merged arm's `n >= 3` guard must fail when the body is
    // absent — no role is surfaced.
    #[test]
    fn merged_arm_guard_rejects_too_few_args() {
        for sub in ["destructor", "initialise", "initialize", "private"] {
            // Only `Target sub` (n == 2, but `sub` is a known subcommand so
            // the bare-script fast path does not apply) → no body role.
            assert!(
                oo_define_arg_roles(&["Target", sub]).is_empty(),
                "{sub}: arity-2 form must not surface a body role"
            );
        }
    }
}
