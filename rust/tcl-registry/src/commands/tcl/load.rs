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

//! `load` — load a shared library extension and initialize new commands.
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[
    // Reads fileName off disk (dlopen/LoadLibrary) before anything else
    // happens.
    SideEffect {
        target: SideEffectTarget::FileIo,
        reads: true,
        ..SideEffect::DEFAULT
    },
    // Registers the library's new commands with the target interpreter.
    SideEffect {
        target: SideEffectTarget::InterpState,
        writes: true,
        ..SideEffect::DEFAULT
    },
];

// `-global`/`-lazy`/`--` and the `prefix` argument name are Tcl 8.6+ only.
// The 8.4 and 8.5 manpages document a plain three-line synopsis with no
// options at all (`load fileName`, `load fileName packageName`, `load
// fileName packageName interp`) and call the second argument
// `packageName`; the 8.6, 9.0, and 9.1 manpages all add the `?-global?
// ?-lazy? ?--?` option prefix and rename the argument to `prefix` (raw
// manpage HTML checked for every version — Tcl 9.1's load.html is
// byte-for-byte identical to 9.0's apart from the C-level
// `Tcl_CreateObjCommand2`/`Tcl_Size` signature in its EXAMPLE code, which
// doesn't touch the Tcl-level command).
const FORMS: &[FormSpec] = &[
    FormSpec {
        synopsis: "load fileName ?packageName? ?interp?",
        dialects: Some(DialectSet::TCL84.union(DialectSet::TCL85)),
        ..FormSpec::DEFAULT
    },
    FormSpec {
        synopsis: "load ?-global? ?-lazy? ?--? fileName ?prefix? ?interp?",
        dialects: Some(DialectSet::TCL86_PLUS),
        ..FormSpec::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "load",
        // Universal core Tcl 8.4-9.1 (present, with an unchanged basic
        // shape, on every fetched manpage). F5 iRules drops it under the
        // K36322151 bans (no dynamic-linking surface in the TMM data-plane
        // sandbox): this explicit `dialects` group is `ALL_TCL`, which omits
        // the `IRULES` bit, so `load` never intersects the bare `IRULES`
        // mask — the same way `cd`/`open`/`exec` are excluded. There is no
        // disable list anymore; the ban is simply the absent bit. Every
        // other modelled dialect that embeds a real Tcl core (Expect,
        // f5-iapps, f5-tmsh, the EDA vendor shells) has a mask carrying a
        // Tcl-version bit that `ALL_TCL` intersects, so `load` is available
        // there too.
        dialects: Some(DialectSet::ALL_TCL),
        // `TAINT_SINK`: fileName is dlopen'd/LoadLibrary'd unconditionally
        // — attacker-influenced input reaching it is arbitrary native code
        // execution in the process, at least as severe as `exec`'s argv
        // hazard or `open`'s pipe-form hazard (both already modelled as
        // sinks the same way).
        traits: Traits::BYTE_COMPILED
            | Traits::SAFE_INTERP_HIDDEN
            | Traits::TAINT_SINK
            // Binds a shared library's commands into this interpreter: the
            // loaded package's own initialisation script can call straight
            // back into whatever this file has defined, so this file's
            // visible call sites are no longer the complete caller set.
            | Traits::LOADS_EXTERNAL_UNIT,
        // Positional-argument count only: 1 (fileName alone) to 3
        // (fileName + prefix/packageName + interp) in every version.
        // `check_simple_arity` (tcl-compiler) skips leading words that
        // resolve to a declared `OptionSpec` before counting positionals,
        // so the Tcl 8.6+ options never inflate this range: `load -global
        // -lazy -- fileName prefix interp` still counts as exactly 3
        // positionals, the same as 8.4/8.5's option-free `load fileName
        // packageName interp`.
        arity: Arity::new(1, 3),
        return_type: Some(TclType::String),
        // ``--`` is the option terminator that drives W304's
        // ``resolve_option_terminator`` lookup; the registry also
        // surfaces ``-global`` / ``-lazy`` for completion. All three are
        // Tcl 8.6+ only — see FORMS above; the 8.4/8.5 manpages document
        // no options whatsoever for `load`.
        options: const {
            &[
                OptionSpec {
                    name: "-global",
                    value: OptionValue::flag(),
                    detail: "Exports every symbol in the shared library for global use by other loaded libraries, instead of resolving them privately to this library. Ignored (not an error) on platforms that don't support it.",
                    dialects: Some(DialectSet::TCL86_PLUS),
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-lazy",
                    value: OptionValue::flag(),
                    detail: "Delays the actual loading of symbols in the library until they are first used, instead of resolving them all immediately. Ignored (not an error) on platforms that don't support it.",
                    dialects: Some(DialectSet::TCL86_PLUS),
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "Marks the end of the options; needed if fileName begins with - and a prefix argument is also given.",
                    dialects: Some(DialectSet::TCL86_PLUS),
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        hover: Some(HoverSnippet {
            summary: "Load a shared library extension and initialize new commands in an interpreter.",
            synopsis: &[
                "load ?-global? ?-lazy? ?--? fileName",
                "load ?-global? ?-lazy? ?--? fileName prefix",
                "load ?-global? ?-lazy? ?--? fileName prefix interp",
            ],
            snippet: "Loads binary code from fileName — typically a shared library such as a .so or DLL — into the process, then calls an initialization procedure to register new commands with interp (the invoking interpreter, by default). The procedure name comes from prefix: prefix_Init normally, or prefix_SafeInit when interp is a safe interpreter. When prefix is omitted or empty, Tcl guesses it from the last path element of fileName — strip a leading \"lib\", and (from Tcl 9.0) a following \"tcl9\", then title-case the remaining word characters; load libxyz4.2.so guesses Xyz. fileName may be empty only when prefix names a package already registered as statically linked. A given fileName's code is loaded into the process at most once — loading it again, even into a different interp, just re-invokes the initialization procedure without reloading. Before Tcl 8.6, load took no options and called its second argument packageName rather than prefix; -global/-lazy/-- exist only from 8.6 on, and misusing either can crash the process later (a symbol conflict or a missing symbol) in a way load itself never detects or reports. Before Tcl 9.0 an explicitly-supplied prefix was itself title-cased before use, the same as a guessed one; from 9.0 an explicit prefix is used exactly as given, so its case now matters. Before Tcl 8.5 a load could never be undone; from 8.5 the unload command can remove a library that cooperates with Tcl's unloading protocol.",
            source: "Tcl load(n)",
            examples: "load [file join [pwd] libfoo[info sharedlibextension]]\nload -lazy [file join [pwd] libfoo[info sharedlibextension]] Foo ;# Tcl 8.6+: -global/-lazy/-- added in 8.6\nload {} Tk ;# fileName empty: use a package already registered as statically linked",
            return_value: "The result returned by the library's initialization procedure — typically the empty string on success.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Load),
        ..CommandSpec::DEFAULT
    }
}
