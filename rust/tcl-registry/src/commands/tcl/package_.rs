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

//! `package` — facilities for package loading and version control.

use crate::prelude::*;
use tcl_dialect::model::Family;
use tcl_dialect::model::SpecSurface;
use tcl_dialect::surface;

use crate::model::binding::PackageTransition;

const PACKAGE_DOMAINS: &[StateTransitionDomain] = &[StateTransitionDomain::Packages];

// Tcl_PackageObjCmd's ifneeded branch retains the script in the package
// database; it never evaluates that script during registration or lookup.
// The future require operation keeps its independent open callback effect.
const PACKAGE_REGISTRATION_EFFECTS: WorldEffectDescriptor = WorldEffectDescriptor {
    static_footprint: StaticEffectFootprint {
        accesses: &[StaticEffectAccess::new(
            WorldStateDomain::PackageState,
            EffectAccessMode::ReadWrite,
            StaticInterpreterScope::Current,
            StaticNamespaceScope::Any,
            StaticSubjectScope::Wildcard,
        )],
        callback: CallbackEffect::NONE,
    },
    ..WorldEffectDescriptor::EMPTY
};

const fn package_descriptor(resolver: StateTransitionResolver) -> StateTransitionDescriptor {
    StateTransitionDescriptor {
        logical_source_resolver: None,
        composition: StateTransitionComposition::Extend,
        success_resolver: None,
        resolver: Some(resolver),
        argument_shape: StateTransitionArgumentShape::Positional,
        dynamic_widening: &[StateTransitionWideningRule {
            operands: StateTransitionOperandLayout::EveryArgument,
            domains: PACKAGE_DOMAINS,
        }],
        effect_coverage: &[],
        commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
    }
}

fn package_transition(transition: PackageTransition) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    transitions.push(StateTransition::Package(transition));
    transitions
}

fn provide_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    if arguments.len() != 3 {
        return StateTransitions::default();
    }
    let (Some(package), Some(version)) = (
        TransitionSubject::from_argument(arguments, 1),
        TransitionSubject::from_argument(arguments, 2),
    ) else {
        return StateTransitions::default();
    };
    if let (Some(version), Some(release)) = (
        arguments.literal_at(2),
        arguments.dialect().and_then(|dialect| dialect.tcl_version),
    ) && !tcl_dialect::validate_version_for(version, release)
    {
        return StateTransitions::default();
    }
    package_transition(PackageTransition::Provide {
        package,
        version: Some(version),
    })
}

fn require_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    let Some((relative_package, exact)) = package_require_layout(arguments.slice_from(1)) else {
        let mut transitions = StateTransitions::default();
        if let Some(word) = arguments.get(1) {
            transitions.push(StateTransition::Widen(
                crate::state_transition::StateTransitionWidening {
                    domains: PACKAGE_DOMAINS.to_vec(),
                    subject: TransitionSubject::Unknown {
                        argument_index: 1,
                        word_kind: word.kind(),
                    },
                },
            ));
        }
        return transitions;
    };
    let package_index = relative_package + 1;
    if arguments.len() <= package_index
        || (exact && arguments.len() != package_index + 2)
        || arguments
            .dialect()
            .and_then(|dialect| dialect.tcl_version)
            .is_some_and(|version| {
                !version.has_package_requirements() && arguments.len() > package_index + 2
            })
    {
        return StateTransitions::default();
    }
    if let Some(release) = arguments.dialect().and_then(|dialect| dialect.tcl_version) {
        for index in package_index + 1..arguments.len() {
            let word = arguments.get(index);
            let requirement = word.and_then(|word| {
                word.literal()
                    .map(str::as_bytes)
                    .or_else(|| word.native_bytes())
            });
            if let Some(requirement) = requirement
                && ((exact && !tcl_dialect::validate_version_bytes_for(requirement, release))
                    || (!exact
                        && tcl_dialect::validate_requirement_bytes_for(requirement, release)
                            .is_err()))
            {
                return StateTransitions::default();
            }
        }
    }
    let Some(package) = TransitionSubject::from_argument(arguments, package_index) else {
        return StateTransitions::default();
    };
    let requirements = (package_index + 1..arguments.len())
        .filter_map(|index| TransitionSubject::from_argument(arguments, index))
        .collect();
    package_transition(PackageTransition::Require {
        package,
        requirements,
        exact,
    })
}

fn ifneeded_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    if arguments.len() != 4 {
        return StateTransitions::default();
    }
    let (Some(package), Some(version), Some(script)) = (
        TransitionSubject::from_argument(arguments, 1),
        TransitionSubject::from_argument(arguments, 2),
        TransitionSubject::from_argument(arguments, 3),
    ) else {
        return StateTransitions::default();
    };
    package_transition(PackageTransition::Ifneeded {
        package,
        version,
        script_provided: true,
        script: Some(script),
    })
}

fn forget_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    if arguments.len() < 2 {
        return StateTransitions::default();
    }
    package_transition(PackageTransition::Forget {
        packages: (1..arguments.len())
            .filter_map(|index| TransitionSubject::from_argument(arguments, index))
            .collect(),
    })
}

fn unknown_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    if arguments.len() != 2 {
        return StateTransitions::default();
    }
    package_transition(PackageTransition::UnknownHandler {
        handler: TransitionSubject::from_argument(arguments, 1),
    })
}

fn prefer_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    if arguments.len() != 2 {
        return StateTransitions::default();
    }
    package_transition(PackageTransition::Prefer {
        mode: TransitionSubject::from_argument(arguments, 1),
    })
}

/// `package unknown prefix` stores the fallback for a later unsatisfied
/// `package require`; the zero-argument form is only a query.
fn package_unknown_script_timing(args: crate::InvocationArguments<'_>) -> Vec<(u8, ScriptTiming)> {
    let Some(count) = args.exact_argv_len() else {
        return Vec::new();
    };
    (count != 0)
        .then_some((0, ScriptTiming::Deferred))
        .into_iter()
        .collect()
}

/// `package require` / `package present`'s one option. `PkgRequireCore`
/// (`generic/tclPkg.c`) compares the word after the subcommand with
/// `strcmp(…, "-exact")`, so the spelling is exact — tclsh 8.4–9.0: `package
/// require -e Tcl` asks for a package named `-e` — and the two subcommands
/// declare their tables [`PrefixMatching::Strict`].
const EXACT_OPTION: &[OptionSpec] = &[OptionSpec {
    name: "-exact",
    value: OptionValue::flag(),
    detail: "Restricts the match to exactly the given version — a single version word, not a requirement list. Equivalent to `package require package version-version`; a different already-provided version is a hard error.",
    surface: None,
    aliases: &[],
    lifecycle: Lifecycle::UNSPECIFIED,
    min_abbrev: None,
    effect: None,
}];

/// One optional exact flag, not a repeatable option run. This is the selected
/// package descriptor's source layout, shared by roles and transitions. Byte
/// values stay bytes; NUL-dependent C-string extent is unavailable here.
fn package_require_layout(args: InvocationArguments<'_>) -> Option<(usize, bool)> {
    let count = args.exact_argv_len()?;
    let first = args.get(0)?;
    let exact = match first {
        crate::InvocationWord::Literal(value) => {
            if value.contains('\0') {
                return None;
            }
            EXACT_OPTION[0].matches(value)
        }
        crate::InvocationWord::KnownBytes(value) => {
            if value.contains(&0) {
                return None;
            }
            value == EXACT_OPTION[0].name.as_bytes()
                || EXACT_OPTION[0]
                    .aliases
                    .iter()
                    .any(|alias| value == alias.as_bytes())
        }
        word if word.proves_non_option() => false,
        _ => return None,
    };
    let package = usize::from(exact);
    (count > package).then_some((package, exact))
}

fn package_require_arg_roles(
    args: InvocationArguments<'_>,
    _options: crate::resolved_invocation::InvocationOptions<'_, '_>,
) -> Option<Vec<(u8, ArgRole)>> {
    let count = args.exact_argv_len()?;
    let (package, exact) = package_require_layout(args)?;
    if exact && count != package + 2 {
        return None;
    }
    (package..count)
        .map(|index| {
            Some((
                u8::try_from(index).ok()?,
                if index == package {
                    ArgRole::Name
                } else {
                    ArgRole::Value
                },
            ))
        })
        .collect()
}

// The command's own wrong-#-args usage message — the generic ensemble
// dispatch shape. NOT actually unchanged across 8.4-9.1: confirmed via
// the real Tcl core source (generic/tclPkg.c's `Tcl_WrongNumArgs(interp,
// 1, objv, …)` for `objc < 2`, fetched from the tcltk/tcl GitHub mirror
// at tags core-8-4-20/core-8-5-19/core-8-6-14/core-9-0-4/core-9-1-b0)
// that Tcl 8.4 and 8.5 both emit the older, doubled-"arg" "option ?arg
// arg ...?" wording, while 8.6 reworded it to "option ?arg ...?" and
// 9.0/9.1 keep 8.6's wording verbatim (also confirmed live via `tclsh`
// 8.6.14). Mirrors `dict.rs`'s `"dict option arg ?arg ...?"` /
// `chan_.rs`'s `"chan subcommand ?arg ...?"`; the two-entry
// split-by-dialect shape mirrors `return_.rs`'s TCL84-only legacy form.
const FORMS: &[FormSpec] = &[
    FormSpec {
        synopsis: "package option ?arg ...?",
        surface: Some(SpecSurface::TCL86_PLUS),
        ..FormSpec::DEFAULT
    },
    FormSpec {
        synopsis: "package option ?arg arg ...?",
        surface: Some(surface![SpecSurface::core_in(
            Family::Tcl,
            &[("8.4", Some("8.6"))]
        )]),
        ..FormSpec::DEFAULT
    },
];

/// `package prefer`'s two legal mode words (Tcl 8.5+, TIP 268) — any other
/// value is a hard error ("raise an invalid argument error" per the
/// manpage), and the argument is always a single bare word, never a list,
/// so (unlike `open`'s `ACCESS_VALUES`) an exact closed-value check is
/// safe here.
const PREFER_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "latest",
        detail: "Accept the highest version satisfying the requirements, regardless of stability.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "stable",
        detail: "Prefer the highest stable version satisfying the requirements; fall back to the highest unstable version if none is stable. The default, unless TCL_PKG_PREFER_LATEST is set in the environment (Tcl 9.0+: or the running Tcl itself is an unstable build).",
        ..ArgValue::DEFAULT
    },
];

/// `package vsatisfies` changed from a two-argument comparison in Tcl 8.4 to
/// an OR-list of requirements in Tcl 8.5 (TIP 268).  Keep the common
/// subcommand arity at the legacy exact shape: the generic resolver falls
/// back to it when no dialect-specific form accepts the call, while Tcl
/// 8.5+ selects the variadic form for a longer requirement list.
///
/// `PKG_VSATISFIES` in `generic/tclPkg.c` (8.4.20 to 9.1.0) only parses and
/// compares its operands, so each form replaces the command-level package
/// database hint with an explicitly empty one.
const VSATISFIES_FORMS: &[SubCommandForm] = &[
    SubCommandForm {
        name: "tcl8.4",
        source_package_query: Some(crate::source_navigation::SourcePackageQuery::VersionSatisfies),
        arity: Arity::exact(2),
        surface: Some(SpecSurface::TCL84),
        side_effects: Some(&[]),
        ..SubCommandForm::DEFAULT
    },
    SubCommandForm {
        name: "tcl8.5+",
        source_package_query: Some(crate::source_navigation::SourcePackageQuery::VersionSatisfies),
        arity: Arity::at_least(2),
        surface: Some(SpecSurface::TCL85_PLUS),
        side_effects: Some(&[]),
        ..SubCommandForm::DEFAULT
    },
];

/// `PKG_PROVIDE` in `generic/tclPkg.c` (8.4.20 to 9.1.0): with one package
/// operand (`objc == 3`) the command only looks the name up in the package
/// table and returns its provided version, or nothing. It never fails, runs
/// no script and changes no state, so this form replaces the parent's
/// registration descriptors with a closed read-only query.
const PROVIDE_FORMS: &[SubCommandForm] = &[SubCommandForm {
    name: "query",
    source_package_query: Some(crate::source_navigation::SourcePackageQuery::ProvidedVersion),
    arity: Arity::exact(1),
    arg_roles: &[(0, ArgRole::Name)],
    side_effects: Some(&[SideEffect {
        target: SideEffectTarget::InterpState,
        reads: true,
        ..SideEffect::DEFAULT
    }]),
    world_effects: Some(WorldEffectDescriptor {
        composition: crate::world_effect::WorldEffectComposition::Replace,
        static_footprint: StaticEffectFootprint {
            accesses: &[StaticEffectAccess::new(
                WorldStateDomain::PackageState,
                EffectAccessMode::Read,
                StaticInterpreterScope::Current,
                StaticNamespaceScope::Any,
                StaticSubjectScope::Wildcard,
            )],
            callback: CallbackEffect::NONE,
        },
        ..WorldEffectDescriptor::EMPTY
    }),
    // Replace, so a dynamic package operand adds no package-state widening.
    state_transitions: Some(StateTransitionDescriptor {
        composition: StateTransitionComposition::Replace,
        ..StateTransitionDescriptor::EMPTY
    }),
    ..SubCommandForm::DEFAULT
}];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "files",
        arity: Arity::exact(1),
        detail: "Lists all files forming part of package, in the order they were sourced during package initialization; auto-loaded files are excluded.",
        synopsis: "package files package",
        pure: true,
        return_type: Some(TclType::List),
        // Added in Tcl 9.0 — absent from the 8.4/8.5/8.6 SYNOPSIS (and the
        // "bad option" error's subcommand list on tclsh 8.6.14), present
        // from 9.0's manpage on (9.1's is byte-for-byte identical to 9.0's
        // apart from the version banner).
        surface: Some(SpecSurface::TCL90_PLUS),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "forget",
        state_transitions: Some(package_descriptor(forget_transitions)),
        arity: Arity::any(),
        detail: "Removes all information about each specified package from this interpreter, clearing both its package ifneeded script(s) and its package provide record.",
        synopsis: "package forget ?package package ...?",
        return_type: Some(TclType::String),
        // Unconditional delete — verified empirically (tclsh 8.6.14) that
        // a bare `package forget` (zero packages) is a silent no-op, not
        // an error, and forgetting never depends on existing state the
        // way `provide`/`ifneeded`/`unknown`'s query forms do. Precise
        // enough to override the command-level (read+write) fallback.
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "ifneeded",
        state_transitions: Some(package_descriptor(ifneeded_transitions)),
        world_effects: Some(PACKAGE_REGISTRATION_EFFECTS),
        arity: Arity::new(2, 3),
        detail: "Registers script to be evaluated (in the global namespace) the next time package require needs this version and it is not yet provided; replaces any script already registered for the same package/version pair. With script omitted, returns the currently registered script, or an empty string if none is registered.",
        synopsis: "package ifneeded package version ?script?",
        return_type: Some(TclType::String),
        // The optional script (index 2 after `ifneeded`) is stored and run
        // later — `uplevel #0 $script` inside `package require`, in the
        // *global* namespace, never the definer's frame — so it is a
        // structural body like `bind`'s/`after`'s deferred scripts, not a
        // caller-frame one: `body_kind: Structural` keeps SSA from scanning
        // it as part of the enclosing `package ifneeded` call's own scope.
        // The package name, the version it registers, and the stored
        // script.
        arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Value), (2, ArgRole::Body)],
        body_kind: BodyKind::Structural,
        body_execution: Some(
            crate::body_execution::BodyExecutionSpec::DeferredGlobalScriptForFamily(Family::Tcl),
        ),
        // `DEFERS_BODY` — "stored and run later" said in data, and scoped to
        // this subcommand rather than to `package` as a whole, since it is the
        // only `package` form with a body. tclsh 8.6.16 and 9.0.4,
        // byte-identical: `proc p {} { package ifneeded x 1.0 {error stop};
        // set ::reached 1 }` sets `::reached`.
        analyser_hook: Some(crate::hooks::AnalyserHookId::PackageIfneeded),
        // Registering a load script names *this* file as the thing that
        // supplies `package`, so the commands it defines are public API a
        // `package require` in some other file reaches — the same
        // caller-set widening `provide` declares, one step earlier.
        traits: Traits::PROVIDES_PACKAGE.union(Traits::DEFERS_BODY),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::exact(0),
        detail: "Returns a list of the names of all packages in the interpreter for which a version has been provided (via package provide) or for which a package ifneeded script is available. The order of elements in the list is arbitrary.",
        synopsis: "package names",
        pure: true,
        return_type: Some(TclType::List),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "prefer",
        state_transitions: Some(package_descriptor(prefer_transitions)),
        arity: Arity::new(0, 1),
        detail: "Returns or sets the current mode of selection logic used by package require/present: \"stable\" (the default) or \"latest\". Once set to latest, an attempt to set it back to stable is silently ineffective.",
        synopsis: "package prefer ?latest|stable?",
        return_type: Some(TclType::String),
        arg_values: &[(0, PREFER_VALUES)],
        closed_value_args: &[0],
        // The setter form writes interpreter-global state that every later
        // `package require` in *any* file reads — which release it selects
        // when the best acceptable version is a prerelease.
        analyser_hook: Some(crate::hooks::AnalyserHookId::PackagePrefer),
        // Added in Tcl 8.5 (TIP 268).
        surface: Some(SpecSurface::TCL85_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "present",
        arity: Arity::at_least(1),
        detail: "Equivalent to package require, except it never attempts to load the package: it only succeeds when a version satisfying the given requirements has already been provided (via package provide) — it does not consult the package ifneeded database or invoke package unknown.",
        synopsis: "package present ?-exact? package ?requirement...?",
        return_type: Some(TclType::String),
        // Verified empirically (tclsh 8.6.14): when the requested version
        // has not already been `package provide`d, `present` raises
        // "... is not present" directly — it never touches the `ifneeded`
        // database (an ifneeded-only, not-yet-provided package still
        // reports "not present") and never invokes `package unknown`.
        // Unlike `require`, it therefore only ever reads existing
        // interpreter state, so it is genuinely side-effect-free.
        pure: true,
        // `-exact` is matched by exact spelling, as `require`'s is.
        prefix_matching: PrefixMatching::Strict,
        options: const {
            &[OptionSpec {
                name: "-exact",
                value: OptionValue::flag(),
                detail: "Restricts the match to exactly the given version — a single version word, not a requirement list. Equivalent to `package present package version-version`; since present never loads, an exact version that has not already been provided is a hard error, not an attempt to load it.",
                surface: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
                effect: None,
            }]
        },
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "provide",
        source_package_query: Some(crate::source_navigation::SourcePackageQuery::ProvidedVersion),
        state_transitions: Some(package_descriptor(provide_transitions)),
        // Reads, and in the setter form writes, only the package database;
        // `Tcl_PkgProvideEx` runs no script or handler.
        world_effects: Some(PACKAGE_REGISTRATION_EFFECTS),
        subcommand_forms: PROVIDE_FORMS,
        arity: Arity::new(1, 2),
        detail: "Declares that version of package is now present in the interpreter; errors if a different version was already provided. With version omitted, returns the currently provided version, or an empty string if package has not been provided.",
        synopsis: "package provide package ?version?",
        return_type: Some(TclType::String),
        // The package name, and the version it now provides.
        arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Value)],
        analyser_hook: Some(crate::hooks::AnalyserHookId::PackageProvide),
        // Declares the file a loadable package: every command it defines
        // is reachable from any file that `package require`s it, so a
        // single-file view of the call sites is never the whole picture.
        traits: Traits::PROVIDES_PACKAGE,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "require",
        source_package_query: Some(crate::source_navigation::SourcePackageQuery::RequiredVersion),
        state_transitions: Some(package_descriptor(require_transitions)),
        arity: Arity::at_least(1),
        detail: "Ensures a version of package satisfying the given requirements is loaded: if not already provided, evaluates the highest-acceptable package ifneeded script (in the global namespace), falling back to package unknown as a last resort. Returns the loaded version; raises an error if no acceptable version becomes available. The highest acceptable version is selected; from Tcl 8.5, this is subject to the package prefer mode (stable by default, preferring a stable version over an unstable one) — Tcl 8.4 has no package prefer and no stable/unstable distinction, so it always simply picks the highest acceptable version.",
        synopsis: "package require ?-exact? package ?requirement...?",
        return_type: Some(TclType::String),
        prefix_matching: PrefixMatching::Strict,
        options: EXACT_OPTION,
        // At most one exact flag precedes the name; later words are requirements.
        arg_role_layout_resolver: Some(package_require_arg_roles),
        arg_role_resolver_roles: &[ArgRole::Name, ArgRole::Value],
        analyser_hook: Some(crate::hooks::AnalyserHookId::PackageRequire),
        // Evaluates another unit's `ifneeded` script in this interpreter,
        // so that unit's top level can call straight back into whatever
        // this file has already defined.
        traits: Traits::LOADS_EXTERNAL_UNIT,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "unknown",
        state_transitions: Some(package_descriptor(unknown_transitions)),
        surface: None,
        arity: Arity::new(0, 1),
        detail: "Sets the last-resort command invoked by package require when no suitable version is found in the ifneeded database. With command omitted, returns the current handler, or an empty string if none is set; an empty-string command clears it.",
        synopsis: "package unknown ?command?",
        return_type: Some(TclType::String),
        // The optional handler (index 0 after `unknown` → arg 1) is a
        // command prefix invoked as `command name requirement ?requirement
        // ...?` when `package require` can't satisfy a package from the
        // ifneeded database. Verified empirically on tclsh 8.6.14 and
        // independently against the real Tcl core source
        // (generic/tclPkg.c's `AddRequirementsToDString`, byte-identical
        // in tags core-8-5-19/core-8-6-14/core-9-0-4/core-9-1-b0): Tcl
        // ALWAYS appends at least *two* words — the package name plus at
        // least one requirement word — even when the triggering `require`
        // call gave no requirement itself (the C source appends a literal
        // `" 0-"` min-unbound placeholder in that case), so AtLeast(2) is
        // the tight floor, not AtLeast(1); the manpage's "If no
        // requirements are supplied ... then only the name will be added"
        // undersells this by one word (a manpage/implementation mismatch
        // unchanged from 8.5 through 9.1.0). Tcl 8.4's default
        // build (core-8-4-20's tclPkg.c carries TIP-268 logic too, but
        // only behind an `#ifdef TCL_TIP268` that no standard 8.4
        // configure/build defines) instead appends the package name and
        // version-or-"{}" — two words, three if the triggering call
        // passed `-exact` (a literal extra "-exact" word) — which is
        // likewise always >= 2, so AtLeast(2) holds for every version.
        command_prefixes: &[(0, AppendedArity::AtLeast(2))],
        // naming.callback.original-unknown-handler-lookup-scope
        // docs/design/analysis/name-resolution-proofs/callback-original-unknown-handler-lookup-scope.md
        // Tcl's package loader evaluates the fallback with TCL_EVAL_GLOBAL.
        script_lookup_scope: Some(crate::ScriptLookupScope::GlobalFrame),
        script_timing_resolver: Some(package_unknown_script_timing),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vcompare",
        arity: Arity::exact(2),
        detail: "Compares the two version numbers given by version1 and version2: returns -1 if version1 is earlier, 0 if they are equal, or 1 if version1 is later.",
        synopsis: "package vcompare version1 version2",
        pure: true,
        return_type: Some(TclType::Int),
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "versions",
        arity: Arity::exact(1),
        detail: "Returns a list of all the version numbers of package for which information has been provided by package ifneeded commands — a package known only via package provide, with no matching ifneeded entry, is not included.",
        synopsis: "package versions package",
        pure: true,
        return_type: Some(TclType::List),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vsatisfies",
        source_package_query: Some(crate::source_navigation::SourcePackageQuery::VersionSatisfies),
        // Tcl 8.4 (pre-TIP-268): exactly 2 words (version1 version2) — a
        // single plain-version comparison; the 8.4 manpage's synopsis is
        // literally `package vsatisfies version1 version2`, no `...`. Tcl
        // 8.5+ made it variadic (`version requirement...`, TIP 268) and
        // added the min / min- / min-max requirement grammar.
        // `at_least(2)` is the tight floor for every version (8.4 never
        // accepts fewer *or more* than 2 — confirmed empirically on
        // tclsh 8.6.14 that fewer than 2 is a wrong-#-args error there
        // too); it just doesn't cap the 8.5+ upper bound, which is
        // unbounded.
        arity: Arity::exact(2),
        detail: "Returns 1 if version satisfies at least one of the given requirements, 0 otherwise. From Tcl 8.5 (TIP 268) each requirement may be a bare min (min, or a later release with the same major version), min- (min or later, unbounded), or min-max (min inclusive, max exclusive); before 8.5 vsatisfies took exactly one plain version to compare against, under the same \"later, same major version\" rule.",
        synopsis: "package vsatisfies version requirement ?requirement ...?",
        pure: true,
        // Compares its operands only; no interpreter state or callback.
        world_effects: Some(WorldEffectDescriptor::EMPTY),
        state_transitions: Some(StateTransitionDescriptor::EMPTY),
        return_type: Some(TclType::Boolean),
        subcommand_forms: VSATISFIES_FORMS,
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::DEFAULT
    },
];

fn jim_provide_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    if !(2..=3).contains(&arguments.len()) {
        return StateTransitions::default();
    }
    let Some(package) = TransitionSubject::from_argument(arguments, 1) else {
        return StateTransitions::default();
    };
    package_transition(PackageTransition::Provide {
        package,
        version: Some(TransitionSubject::Literal("1.0".to_owned())),
    })
}

fn jim_require_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    if !(2..=3).contains(&arguments.len()) {
        return StateTransitions::default();
    }
    let Some(package) = TransitionSubject::from_argument(arguments, 1) else {
        return StateTransitions::default();
    };
    package_transition(PackageTransition::Require {
        package,
        requirements: Vec::new(),
        exact: false,
    })
}

const JIM_SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "forget",
        arity: Arity::at_least(1),
        state_transitions: Some(package_descriptor(forget_transitions)),
        synopsis: "package forget package ?package ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "provide",
        arity: Arity::new(1, 2),
        state_transitions: Some(package_descriptor(jim_provide_transitions)),
        synopsis: "package provide name ?version?",
        detail: "Provides the package at version 1.0; the optional version word is ignored. Repeated provide is an error.",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "require",
        arity: Arity::new(1, 2),
        state_transitions: Some(package_descriptor(jim_require_transitions)),
        traits: Traits::LOADS_EXTERNAL_UNIT,
        synopsis: "package require name ?version?",
        detail: "Loads a package by file lookup; the optional version word is ignored.",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::exact(0),
        pure: true,
        return_type: Some(TclType::List),
        synopsis: "package names",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "list",
        arity: Arity::exact(0),
        pure: true,
        return_type: Some(TclType::List),
        synopsis: "package list",
        ..SubCommand::DEFAULT
    },
];

/// Jim's package surface and protocol, measured on the current 0.84 core.
pub fn jim_spec() -> CommandSpec {
    CommandSpec {
        name: "package",
        // Jim's package handler always uses direct command dispatch.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::NoHook,
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        surface: Some(surface![SpecSurface::core_in(
            Family::Jim,
            &[("0.84", None)]
        )]),
        arity: Arity::at_least(1),
        subcommands: JIM_SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}

/// Command spec for `package`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "package",
        // Every pinned C Tcl registration has a NULL compileProc. Package
        // loading protocols retain their independent execution contracts.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::NoHook,
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        runtime_backing: RuntimeBacking::shipped("package"),
        // Universal core Tcl 8.4-9.1 (present, with the shape detailed on each
        // subcommand above, on every fetched manpage). F5 iRules drops it
        // (K36322151 — the TMM data-plane sandbox has no real package-loading
        // surface): its own surface below is `Some(SpecSurface::ALL_TCL)`,
        // which names only Tcl and so is simply absent under iRules — the same
        // fully-explicit, per-spec exclusion `cd`/`open`/`exec`/ `load` now
        // rely on, with no disable list. Every other modelled dialect that
        // embeds a real Tcl core (Expect, f5-iapps, f5-tmsh, the EDA vendor
        // shells) carries a Tcl release that `ALL_TCL` intersects, so
        // `package` is available there too.
        surface: Some(SpecSurface::ALL_TCL),
        traits: Traits::NOT_PROC_FACTORY | Traits::BYTE_COMPILED | Traits::LANGUAGE_KEYWORD,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        // Fallback for the genuinely impure subcommands whose own
        // `side_effects` is empty (`ifneeded`/`provide`/`require`/
        // `unknown`): each of those both *reads* existing package-
        // database state to decide what to do (a conflicting-version
        // check, an already-provided short-circuit, an existing-script
        // replacement) and can *write* it, and which happens can depend
        // on an argument (query vs. setter form) this static declaration
        // can't see — so the safe union is reads+writes, not writes-only.
        // `forget` (unconditional, never reads first) overrides this with
        // its own precise write-only declaration above; the pure getters
        // (`files`/`names`/`present`/`vcompare`/`versions`/`vsatisfies`)
        // bypass this entirely via their `pure: true` short-circuit in
        // `tcl_compiler::side_effects::classify_side_effects`.
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Facilities for package loading and version control",
            synopsis: &[
                "package files package",
                "package forget ?package package ...?",
                "package ifneeded package version ?script?",
                "package names",
                "package present ?-exact? package ?requirement...?",
                "package prefer ?latest|stable?",
                "package provide package ?version?",
                "package require ?-exact? package ?requirement...?",
                "package unknown ?command?",
                "package vcompare version1 version2",
                "package versions package",
                "package vsatisfies version requirement ?requirement ...?",
            ],
            snippet: "This command keeps a simple database of the packages available for use by the current interpreter and how to load them into the interpreter — matching a suitable version to what an application needs and detecting version clashes. Only package require and package provide are typically invoked directly in application scripts; the rest are used by system scripts (commonly generated by pkg_mkIndex) that maintain the database. package files is Tcl 9.0+ only. Tcl 8.5 (TIP 268) added package prefer and reworked require/present/vsatisfies to take a variadic list of requirements (each a min, min-, or min-max version range) instead of a single plain version, and added alpha/beta (\"a\"/\"b\") version modifiers — package require now also picks between the highest stable and highest unstable acceptable version according to the package prefer mode.",
            source: "Tcl package(n)",
            examples: "package require Tcl 8.6\npackage require http\nif {[catch {package require Snack}]} {\n    # optional package unavailable; continue without it\n} else {\n    # Snack is loaded\n}\npackage provide mypkg 1.0",
            return_value: "Depends on the subcommand: require/present return the loaded/matching version (erroring if none is acceptable); provide returns the currently provided version, or an empty string if unset; ifneeded and unknown return the registered script/handler, or an empty string if none is registered (and an empty string otherwise for their setter forms, and for forget); files/names/versions return a list; vcompare returns -1, 0, or 1; vsatisfies returns a boolean; prefer returns \"latest\" or \"stable\".",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use tcl_dialect::model::{Family, SurfaceQuery};

    use crate::CommandRegistry;

    #[test]
    fn package_registration_has_no_native_compiler_hook() {
        let native = super::spec()
            .native_compilation
            .expect("authored registration");
        for version in tcl_dialect::TclVersion::ALL {
            assert_eq!(
                native.compiler_hook_presence(crate::InvocationDialect::for_version(version)),
                Some(false)
            );
        }
        let jim = crate::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        assert_eq!(
            super::jim_spec()
                .native_compilation
                .unwrap()
                .compiler_hook_presence(jim),
            Some(false)
        );
    }

    /// tclsh8.4.20 `package vsatisfies 8.4 8.4 8.5` raises "wrong # args:
    /// should be "package vsatisfies version1 version2""; 8.5.19 through
    /// 9.1.0 answer it.
    #[test]
    fn vsatisfies_requirement_count_follows_each_release_form() {
        use crate::{InvocationDialect, InvocationWords};
        let registry = CommandRegistry::build_default();
        let dialect = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        for (arguments, accepted_on_84) in [
            (&["vsatisfies", "8.4", "8.5"][..], true),
            (&["vsatisfies", "8.4", "8.4", "8.5"][..], false),
        ] {
            let selected = registry
                .resolve_structured_invocation(
                    InvocationWords::literals("package", arguments).with_dialect(dialect),
                    dialect.authoring_query(),
                )
                .resolved()
                .expect("selected vsatisfies");
            for release in tcl_dialect::TclVersion::ALL {
                let query = SurfaceQuery::core(Family::Tcl, release.version_string());
                assert_eq!(
                    selected.arity_accepts_under(&query),
                    Some(accepted_on_84 || release != tcl_dialect::TclVersion::V8_4),
                    "{release:?} {arguments:?}"
                );
            }
        }
    }

    #[test]
    fn package_ifneeded_stores_future_script_without_current_callback_effect() {
        // Implementation contract: naming.package.ifneeded-current-effect-contract
        // docs/design/analysis/name-resolution-proofs/package-ifneeded-current-effect-contract.md
        use crate::{CallbackKinds, InvocationDialect, InvocationWords, WorldStateDomain};
        let registry = CommandRegistry::build_default();
        for release in tcl_dialect::TclVersion::ALL {
            let dialect = InvocationDialect::for_version(release);
            for arguments in [
                &["ifneeded", "p", "1"][..],
                &["ifneeded", "p", "1", "rename ::package {}; error DEFERRED"][..],
            ] {
                let selected = registry
                    .resolve_structured_invocation(
                        InvocationWords::literals("package", arguments).with_dialect(dialect),
                        dialect.authoring_query(),
                    )
                    .resolved()
                    .expect("selected package registration");
                let effects = selected.effects();
                assert!(
                    !effects.requires_world_barrier(),
                    "{release:?} {arguments:?}"
                );
                assert!(!effects.callback().kinds.contains(CallbackKinds::SCRIPT));
                assert!(!effects.callback().kinds.is_unknown());
                assert!(
                    effects
                        .accesses()
                        .iter()
                        .any(|access| access.domain == WorldStateDomain::PackageState)
                );
                assert!(!effects.accesses().iter().any(|access| matches!(
                    access.domain,
                    WorldStateDomain::CommandBindings | WorldStateDomain::NamespaceLookup
                )));
            }
            let require = registry
                .resolve_structured_invocation(
                    InvocationWords::literals("package", &["require", "p"]).with_dialect(dialect),
                    dialect.authoring_query(),
                )
                .resolved()
                .expect("selected package require");
            assert!(require.effects().requires_world_barrier());
            assert!(require.effects().callback().kinds.is_unknown());
        }
    }

    #[test]
    fn package_transition_grammar_is_contextual_and_queries_do_not_mutate() {
        use crate::model::binding::PackageTransition;
        use crate::{InvocationDialect, InvocationWords, StateTransition, TransitionSubject};
        let registry = CommandRegistry::build_default();
        for release in tcl_dialect::TclVersion::ALL {
            let require = registry
                .resolve_structured_invocation(
                    InvocationWords::literals("package", &["require", "p", "1", "2"])
                        .with_dialect(InvocationDialect::for_version(release)),
                    Some(SurfaceQuery::core(Family::Tcl, release.version_string())),
                )
                .invocation
                .expect("package require resolves")
                .facts();
            let loads = require
                .state_transitions
                .declared()
                .expect("declared package transitions")
                .facts()
                .iter()
                .map(|fact| &fact.transition)
                .any(|transition| {
                    matches!(
                        transition,
                        StateTransition::Package(PackageTransition::Require { .. })
                    )
                });
            assert_eq!(loads, release >= tcl_dialect::TclVersion::V8_5);
        }
        for words in [
            &["provide", "p"][..],
            &["ifneeded", "p", "1"][..],
            &["unknown"][..],
        ] {
            let facts = registry
                .resolve_invocation("package", words, None)
                .expect("query resolves")
                .facts();
            assert!(
                facts
                    .state_transitions
                    .declared()
                    .expect("declared package transitions")
                    .facts()
                    .is_empty()
            );
        }
        let facts = registry
            .resolve_invocation("package", &["ifneeded", "p", "1", "set ::loaded 1"], None)
            .expect("registration resolves")
            .facts();
        assert!(facts.state_transitions.declared().expect("declared package transitions").facts().iter().map(|fact| &fact.transition).any(|transition| matches!(transition, StateTransition::Package(PackageTransition::Ifneeded { script: Some(TransitionSubject::LocatedLiteral { value: script, .. }), .. }) if script == "set ::loaded 1")));
        let jim = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )));
        let registry = registry.project_for_profile(jim);
        assert!(
            registry
                .get("package")
                .expect("Jim package")
                .subcommand("ifneeded")
                .is_none()
        );
        assert!(
            registry
                .get("package")
                .expect("Jim package")
                .subcommand("require")
                .expect("require")
                .arity
                .accepts(2)
        );
    }

    /// `generic/tclPkg.c` 8.4.20 to 9.1.0: `package provide NAME` is a hash
    /// lookup that returns the provided version or nothing, and
    /// `package vsatisfies` only compares; `package provide NAME VERSION`
    /// stays a registration.
    #[test]
    fn provide_query_and_vsatisfies_are_closed_read_only_selections() {
        use crate::source_navigation::SourcePackageQuery;
        use crate::world_effect::EffectAccessMode;
        use crate::{
            CallbackKinds, InvocationDialect, InvocationWord, InvocationWords, WorldStateDomain,
        };
        let registry = CommandRegistry::build_default();
        for release in tcl_dialect::TclVersion::ALL {
            let dialect = InvocationDialect::for_version(release);
            let select = |words: InvocationWords<'_>| {
                registry
                    .resolve_structured_invocation(
                        words.with_dialect(dialect),
                        dialect.authoring_query(),
                    )
                    .resolved()
                    .expect("selected package operation")
                    .facts()
            };
            let mut closed_reads = vec![
                select(InvocationWords::literals("package", &["provide", "Tcl"])),
                select(InvocationWords::structured(
                    InvocationWord::Literal("package"),
                    &[InvocationWord::Literal("provide"), InvocationWord::Dynamic],
                )),
                select(InvocationWords::literals(
                    "package",
                    &["vsatisfies", "8.6", "8.5"],
                )),
            ];
            if release != tcl_dialect::TclVersion::V8_4 {
                closed_reads.push(select(InvocationWords::literals(
                    "package",
                    &["vsatisfies", "8.6", "8.5", "9"],
                )));
            }
            for facts in closed_reads {
                assert!(
                    facts
                        .state_transitions
                        .declared()
                        .expect("closed transitions")
                        .facts()
                        .is_empty(),
                    "{release:?}"
                );
                assert_eq!(facts.effects.callback().kinds, CallbackKinds::NONE);
                assert!(
                    facts
                        .effects
                        .accesses()
                        .iter()
                        .all(|access| access.mode == EffectAccessMode::Read),
                    "{release:?}"
                );
            }
            let query = registry
                .resolve_structured_invocation(
                    InvocationWords::literals("package", &["provide", "Tcl"]).with_dialect(dialect),
                    dialect.authoring_query(),
                )
                .resolved()
                .expect("selected provide query");
            assert_eq!(
                query
                    .authored_source_package_query()
                    .map(|selected| selected.query),
                Some(SourcePackageQuery::ProvidedVersion)
            );
            let setter = select(InvocationWords::literals(
                "package",
                &["provide", "p", "1.0"],
            ));
            assert!(
                setter
                    .effects
                    .accesses()
                    .iter()
                    .any(|access| access.domain == WorldStateDomain::PackageState
                        && access.mode == EffectAccessMode::ReadWrite)
            );
            assert!(
                !setter
                    .state_transitions
                    .declared()
                    .expect("declared package transitions")
                    .facts()
                    .is_empty()
            );
        }
    }

    #[test]
    fn jim_package_inventory_keeps_hidden_list_and_excludes_ifneeded() {
        // naming.package.jim-source-operation-table
        // docs/design/analysis/name-resolution-proofs/package-jim-source-operation-table.md
        // naming.package.jim-hidden-list
        // docs/design/analysis/name-resolution-proofs/package-jim-hidden-list.md
        // naming.package.ifneeded-native-entry-frame
        // docs/design/analysis/name-resolution-proofs/package-ifneeded-native-entry-frame.md
        // Authored catalogue coverage; this does not execute the native provider.
        let dialect = crate::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        );
        let registry = CommandRegistry::build_default();
        let package = registry
            .get_for_surface("package", dialect.authoring_query())
            .expect("genuine Jim package catalogue");
        let names = package
            .subcommands
            .iter()
            .map(|operation| operation.name)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            names,
            std::collections::BTreeSet::from(["forget", "provide", "require", "list", "names"])
        );
        assert!(package.subcommand("ifneeded").is_none());
        for name in ["list", "names"] {
            let operation = package.subcommand(name).expect("supported operation");
            assert!(operation.arity.accepts(0));
            assert!(!operation.arity.accepts(1));
            assert_eq!(operation.return_type, Some(crate::TclType::List));
            assert!(operation.arg_roles.is_empty());
            assert!(operation.body_execution.is_none());
        }
    }

    #[test]
    fn vsatisfies_arity_is_dialect_aware() {
        let registry = CommandRegistry::build_default();
        let legacy = registry
            .resolve_call(
                "package",
                &["vsatisfies", "8.4", "8.4", "9.0"],
                Some(SurfaceQuery::core(Family::Tcl, "8.4")),
            )
            .expect("package is in the Tcl 8.4 registry");
        assert!(legacy.form.is_none());
        assert!(!legacy.arity().accepts(3));

        let modern = registry
            .resolve_call(
                "package",
                &["vsatisfies", "8.4", "8.4", "9.0"],
                Some(SurfaceQuery::core(Family::Tcl, "8.5")),
            )
            .expect("package is in the Tcl 8.5 registry");
        assert_eq!(modern.form.map(|form| form.name), Some("tcl8.5+"));
        assert!(modern.arity().accepts(3));
    }
}

#[cfg(test)]
mod single_package_option_layout_tests {
    use super::*;

    #[test]
    fn repeated_exact_is_the_package_name_after_one_flag() {
        assert_eq!(
            package_require_layout(InvocationArguments::literals(&["-exact", "-exact", "1.0"])),
            Some((1, true))
        );
        assert_eq!(
            package_require_layout(InvocationArguments::literals(&["-e", "1.0"])),
            Some((0, false))
        );
        let words = [
            crate::InvocationWord::KnownBytes(b"-exact"),
            crate::InvocationWord::KnownBytes(b"-exact"),
            crate::InvocationWord::KnownBytes(b"1.0"),
        ];
        assert_eq!(
            package_require_layout(InvocationArguments::structured(&words)),
            Some((1, true))
        );
        let transitions = require_transitions(InvocationArguments::structured(&[
            crate::InvocationWord::Literal("require"),
            words[0],
            words[1],
            words[2],
        ]));
        assert!(transitions.facts().iter().any(|fact| matches!(&fact.transition,
            StateTransition::Package(PackageTransition::Require { package: TransitionSubject::LocatedNativeBytes { value, argument_index: 2 }, exact: true, .. }) if value == b"-exact")));
    }

    #[test]
    fn byte_requirements_keep_the_original_release_distinction() {
        let words = [
            crate::InvocationWord::Literal("require"),
            crate::InvocationWord::KnownBytes(b"p"),
            crate::InvocationWord::KnownBytes(b"1"),
            crate::InvocationWord::KnownBytes(b"2"),
        ];
        for release in tcl_dialect::TclVersion::ALL {
            let transitions = require_transitions(
                InvocationArguments::structured(&words)
                    .with_dialect(crate::InvocationDialect::for_version(release)),
            );
            let loads = transitions.facts().iter().any(|fact| {
                matches!(
                    fact.transition,
                    StateTransition::Package(PackageTransition::Require { .. })
                )
            });
            assert_eq!(loads, release >= tcl_dialect::TclVersion::V8_5);
        }
    }

    #[test]
    fn unknown_option_extent_never_becomes_a_positional_name() {
        for word in [
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Expanded,
            crate::InvocationWord::KnownBytes(b"-exact\0suffix"),
        ] {
            assert_eq!(
                package_require_layout(InvocationArguments::structured(&[
                    word,
                    crate::InvocationWord::Literal("name")
                ])),
                None
            );
        }
    }
}
