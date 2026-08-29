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

//! `option` command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        arity: Arity::new(2, 3),
        detail: "Add an option to the database with optional priority.",
        synopsis: "option add pattern value ?priority?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "clear",
        arity: Arity::exact(0),
        detail: "Clear all options from the database.",
        synopsis: "option clear",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "get",
        arity: Arity::exact(3),
        detail: "Retrieve the value of the option for a window.",
        synopsis: "option get window name class",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "readfile",
        arity: Arity::new(1, 2),
        detail: "Read options from a file and add them to the database.",
        synopsis: "option readfile fileName ?priority?",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "option option ?arg ...?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "option",
        surface: Some(SpecSurface::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Add or retrieve window options to or from the option database.",
            synopsis: &[
                "option add pattern value ?priority?",
                "option clear",
                "option get window name class",
                "option readfile fileName ?priority?",
            ],
            snippet: "",
            source: "Tk man page option.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}
