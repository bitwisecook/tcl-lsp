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

/// `package unknown prefix` stores the fallback for a later unsatisfied
/// `package require`; the zero-argument form is only a query.
fn package_unknown_script_timing(args: &[&str]) -> Vec<(u8, ScriptTiming)> {
    (!args.is_empty())
        .then_some((0, ScriptTiming::Deferred))
        .into_iter()
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
        dialects: Some(DialectSet::TCL86_PLUS),
        ..FormSpec::DEFAULT
    },
    FormSpec {
        synopsis: "package option ?arg arg ...?",
        dialects: Some(DialectSet::TCL84.union(DialectSet::TCL85)),
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
const VSATISFIES_FORMS: &[SubCommandForm] = &[
    SubCommandForm {
        name: "tcl8.4",
        arity: Arity::exact(2),
        dialects: Some(DialectSet::TCL84),
        ..SubCommandForm::DEFAULT
    },
    SubCommandForm {
        name: "tcl8.5+",
        arity: Arity::at_least(2),
        dialects: Some(DialectSet::TCL85_PLUS),
        ..SubCommandForm::DEFAULT
    },
];

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
        dialects: Some(DialectSet::TCL90_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "forget",
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
        arg_roles: &[(2, ArgRole::Body)],
        body_kind: BodyKind::Structural,
        // `DEFERS_BODY` — "stored and run later" said in data, and scoped to
        // this subcommand rather than to `package` as a whole, since it is the
        // only `package` form with a body. tclsh 8.6.16 and 9.0.4,
        // byte-identical: `proc p {} { package ifneeded x 1.0 {error stop};
        // set ::reached 1 }` sets `::reached` (issue #1672 audit).
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
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "prefer",
        arity: Arity::new(0, 1),
        detail: "Returns or sets the current mode of selection logic used by package require/present: \"stable\" (the default) or \"latest\". Once set to latest, an attempt to set it back to stable is silently ineffective.",
        synopsis: "package prefer ?latest|stable?",
        return_type: Some(TclType::String),
        arg_values: &[(0, PREFER_VALUES)],
        closed_value_args: &[0],
        // The setter form writes interpreter-global state that every later
        // `package require` in *any* file reads — which release it selects
        // when the best acceptable version is a prerelease (issue #1126
        // item 1).
        analyser_hook: Some(crate::hooks::AnalyserHookId::PackagePrefer),
        // Added in Tcl 8.5 (TIP 268).
        dialects: Some(DialectSet::TCL85_PLUS),
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
        options: const {
            &[OptionSpec {
                name: "-exact",
                value: OptionValue::flag(),
                detail: "Restricts the match to exactly the given version — a single version word, not a requirement list. Equivalent to `package present package version-version`; since present never loads, an exact version that has not already been provided is a hard error, not an attempt to load it.",
                dialects: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "provide",
        arity: Arity::new(1, 2),
        detail: "Declares that version of package is now present in the interpreter; errors if a different version was already provided. With version omitted, returns the currently provided version, or an empty string if package has not been provided.",
        synopsis: "package provide package ?version?",
        return_type: Some(TclType::String),
        analyser_hook: Some(crate::hooks::AnalyserHookId::PackageProvide),
        // Declares the file a loadable package: every command it defines
        // is reachable from any file that `package require`s it, so a
        // single-file view of the call sites is never the whole picture.
        traits: Traits::PROVIDES_PACKAGE,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "require",
        arity: Arity::at_least(1),
        detail: "Ensures a version of package satisfying the given requirements is loaded: if not already provided, evaluates the highest-acceptable package ifneeded script (in the global namespace), falling back to package unknown as a last resort. Returns the loaded version; raises an error if no acceptable version becomes available. The highest acceptable version is selected; from Tcl 8.5, this is subject to the package prefer mode (stable by default, preferring a stable version over an unstable one) — Tcl 8.4 has no package prefer and no stable/unstable distinction, so it always simply picks the highest acceptable version.",
        synopsis: "package require ?-exact? package ?requirement...?",
        return_type: Some(TclType::String),
        options: const {
            &[OptionSpec {
                name: "-exact",
                value: OptionValue::flag(),
                detail: "Restricts the match to exactly the given version — a single version word, not a requirement list. Equivalent to `package require package version-version`; a different already-provided version is a hard error.",
                dialects: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
        analyser_hook: Some(crate::hooks::AnalyserHookId::PackageRequire),
        // Evaluates another unit's `ifneeded` script in this interpreter,
        // so that unit's top level can call straight back into whatever
        // this file has already defined.
        traits: Traits::LOADS_EXTERNAL_UNIT,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "unknown",
        dialects: None,
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
        // unchanged from 8.5 through the 9.1 beta). Tcl 8.4's default
        // build (core-8-4-20's tclPkg.c carries TIP-268 logic too, but
        // only behind an `#ifdef TCL_TIP268` that no standard 8.4
        // configure/build defines) instead appends the package name and
        // version-or-"{}" — two words, three if the triggering call
        // passed `-exact` (a literal extra "-exact" word) — which is
        // likewise always >= 2, so AtLeast(2) holds for every version.
        command_prefixes: &[(0, AppendedArity::AtLeast(2))],
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
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "versions",
        arity: Arity::exact(1),
        detail: "Returns a list of all the version numbers of package for which information has been provided by package ifneeded commands — a package known only via package provide, with no matching ifneeded entry, is not included.",
        synopsis: "package versions package",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vsatisfies",
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
        synopsis: "package vsatisfies version requirement...",
        pure: true,
        return_type: Some(TclType::Boolean),
        subcommand_forms: VSATISFIES_FORMS,
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `package`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "package",
        // Universal core Tcl 8.4-9.1 (present, with the shape detailed on
        // each subcommand above, on every fetched manpage). F5 iRules
        // drops it (K36322151 — the TMM data-plane sandbox has no real
        // package-loading surface): its own `dialects` group below is
        // `Some(DialectSet::ALL_TCL)`, which carries no `IRULES` bit and
        // so never intersects the bare `IRULES` availability mask — the
        // same fully-explicit, per-spec exclusion `cd`/`open`/`exec`/
        // `load` now rely on, with no disable list. Every other modelled
        // dialect that embeds a real Tcl core (Expect, f5-iapps, f5-tmsh,
        // the EDA vendor shells) carries a Tcl-version bit that `ALL_TCL`
        // intersects, so `package` is available there too.
        dialects: Some(DialectSet::ALL_TCL),
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
                "package vsatisfies version requirement...",
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
    use crate::{CommandRegistry, dialects::DialectSet};

    #[test]
    fn vsatisfies_arity_is_dialect_aware() {
        let registry = CommandRegistry::build_default();
        let legacy = registry
            .resolve_call(
                "package",
                &["vsatisfies", "8.4", "8.4", "9.0"],
                DialectSet::TCL84,
            )
            .expect("package is in the Tcl 8.4 registry");
        assert!(legacy.form.is_none());
        assert!(!legacy.arity().accepts(3));

        let modern = registry
            .resolve_call(
                "package",
                &["vsatisfies", "8.4", "8.4", "9.0"],
                DialectSet::TCL85,
            )
            .expect("package is in the Tcl 8.5 registry");
        assert_eq!(modern.form.map(|form| form.name), Some("tcl8.5+"));
        assert!(modern.arity().accepts(3));
    }
}
