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

//! `tcl::tm::path` command (Tcl Modules, TIP 189 — added in Tcl 8.5).
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        arity: Arity::at_least(0),
        analyser_hook: Some(AnalyserHookId::ModulePathAdd),
        detail: "",
        synopsis: "",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "list",
        arity: Arity::exact(0),
        detail: "",
        synopsis: "",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "remove",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::tm::path",
        surface: Some(SpecSurface::TCL85_PLUS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Manage the list of paths searched for Tcl modules.",
            synopsis: &[
                "tcl::tm::path add ?path ...?",
                "tcl::tm::path remove ?path ...?",
                "tcl::tm::path list",
            ],
            snippet: "The ``add`` subcommand prepends paths to the module search list, ``remove`` deletes them, and ``list`` returns the current list.",
            source: "Tcl stdlib tm module system",
            examples: "",
            return_value: "",
        }),
        side_effects: crate::knowledge::Knowledge::declared(SIDE_EFFECTS),
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}

const LEGACY_PATH_OPERAND: &[crate::arity::ArityWindow] = &[crate::arity::ArityWindow {
    lifecycle: Lifecycle::UNSPECIFIED.retired_from("8.6"),
    arity: Arity::at_least(1),
}];

const fn member(
    name: &'static str,
    arity: Arity,
    arity_windows: &'static [crate::arity::ArityWindow],
    hover: HoverSnippet,
) -> CommandSpec {
    CommandSpec {
        name,
        surface: Some(SpecSurface::TCL85_PLUS),
        arity,
        arity_windows,
        hover: Some(hover),
        ..CommandSpec::DEFAULT
    }
}

/// Direct implementations behind the public `tcl::tm::path` ensemble.
pub fn member_specs() -> [CommandSpec; 3] {
    [
        CommandSpec {
            analyser_hook: Some(AnalyserHookId::ModulePathAdd),
            ..member(
                "tcl::tm::add",
                Arity::any(),
                LEGACY_PATH_OPERAND,
                HoverSnippet::brief(
                    "Prepend directories to the Tcl module search path.",
                    &["tcl::tm::add ?path ...?"],
                    "Tcl tm(n)",
                ),
            )
        },
        member(
            "tcl::tm::list",
            Arity::exact(0),
            &[],
            HoverSnippet::brief(
                "Return the Tcl module search path.",
                &["tcl::tm::list"],
                "Tcl tm(n)",
            ),
        ),
        member(
            "tcl::tm::remove",
            Arity::any(),
            LEGACY_PATH_OPERAND,
            HoverSnippet::brief(
                "Remove directories from the Tcl module search path.",
                &["tcl::tm::remove ?path ...?"],
                "Tcl tm(n)",
            ),
        ),
    ]
}
