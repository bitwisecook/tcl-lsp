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

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "package files package",
}];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "files",
        arity: Arity::exact(1),
        detail: "Lists all files forming part of package.",
        synopsis: "package files package",
        pure: true,
        return_type: Some(TclType::List),
        dialects: Some(DialectSet::TCL90_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "forget",
        arity: Arity::any(),
        detail: "Removes all information about each specified package from this interpreter.",
        synopsis: "package forget ?package package ...?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "ifneeded",
        arity: Arity::new(2, 3),
        detail: "Set up or query the package ifneeded script.",
        synopsis: "package ifneeded package version ?script?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::exact(0),
        detail: "Returns a list of the names of all packages in the interpreter.",
        synopsis: "package names",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "prefer",
        arity: Arity::new(0, 1),
        detail: "Returns or sets the current mode of selection logic used by package require.",
        synopsis: "package prefer ?latest|stable?",
        return_type: Some(TclType::String),
        // Added in Tcl 8.5 (TIP 268).
        dialects: Some(DialectSet::TCL85_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "present",
        arity: Arity::at_least(1),
        detail: "Equivalent to package require except that it does not try to load the package if it is not already loaded.",
        synopsis: "package present ?-exact? package ?requirement...?",
        return_type: Some(TclType::String),
        options: const {
            &[OptionSpec {
                name: "-exact",
                value: OptionValue::flag(),
                detail: "",
                dialects: None,
                aliases: &[],
                min_version: None,
            }]
        },
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "provide",
        arity: Arity::new(1, 2),
        detail: "Indicates that a version of a package is now present in the interpreter.",
        synopsis: "package provide package ?version?",
        return_type: Some(TclType::String),
        analyser_hook: Some(crate::hooks::AnalyserHookId::PackageProvide),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "require",
        arity: Arity::at_least(1),
        detail: "Load a package, finding and sourcing the appropriate script if needed.",
        synopsis: "package require package ?requirement...?",
        return_type: Some(TclType::String),
        options: const {
            &[OptionSpec {
                name: "-exact",
                value: OptionValue::flag(),
                detail: "",
                dialects: None,
                aliases: &[],
                min_version: None,
            }]
        },
        analyser_hook: Some(crate::hooks::AnalyserHookId::PackageRequire),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "unknown",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        arity: Arity::new(0, 1),
        detail: "Supplies a command to invoke during package require if no suitable version can be found.",
        synopsis: "package unknown ?command?",
        return_type: Some(TclType::String),
        // The optional handler (index 0 after `unknown` → arg 1) is a command
        // prefix invoked as `command name ?requirement ...?` when a package is
        // missing (AtLeast(1): the package name is always passed).
        command_prefixes: &[(0, AppendedArity::AtLeast(1))],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vcompare",
        arity: Arity::exact(2),
        detail: "Compares the two version numbers given by version1 and version2.",
        synopsis: "package vcompare version1 version2",
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "versions",
        arity: Arity::exact(1),
        detail: "Returns a list of all the version numbers of package for which information has been provided.",
        synopsis: "package versions package",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vsatisfies",
        arity: Arity::at_least(2),
        detail: "Returns 1 if the version satisfies at least one of the given requirements, and 0 otherwise.",
        synopsis: "package vsatisfies version requirement...",
        pure: true,
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `package`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "package",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::WASM_EMITS_NOTHING,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Facilities for package loading and version control",
            synopsis: &[
                "package files package",
                "package forget ?package package ...?",
                "package ifneeded package version ?script?",
                "package names",
                "package provide package ?version?",
                "package require package ?requirement...?",
            ],
            snippet: "This command keeps a simple database of the packages available for use by the current interpreter and how to load them into the interpreter.",
            source: "Tcl man page package.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
