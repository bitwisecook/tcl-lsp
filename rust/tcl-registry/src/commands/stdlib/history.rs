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

//! `history` command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        arity: Arity::new(1, 2),
        detail: "Add a command to the history list.",
        synopsis: "history add command ?exec?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "change",
        arity: Arity::new(1, 2),
        detail: "Replace a history event.",
        synopsis: "history change newValue ?event?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "clear",
        arity: Arity::exact(0),
        detail: "Clear the history list.",
        synopsis: "history clear",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "event",
        surface: None,
        arity: Arity::new(0, 1),
        detail: "Return a history event by number or pattern.",
        synopsis: "history event ?event?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::new(0, 1),
        detail: "Return a formatted history list.",
        synopsis: "history info ?count?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "keep",
        arity: Arity::new(0, 1),
        detail: "Get or set the size of the history list.",
        synopsis: "history keep ?count?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "nextid",
        arity: Arity::exact(0),
        detail: "Return the next event number.",
        synopsis: "history nextid",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "redo",
        arity: Arity::new(0, 1),
        detail: "Re-evaluate a history event.",
        synopsis: "history redo ?event?",
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
    synopsis: "history subcommand ?arg ...?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "history",
        traits: Traits::UNSAFE | Traits::OVERRIDABLE_LIBRARY_PROC,
        surface: Some(SpecSurface::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Manipulate the history list of previously executed commands.",
            synopsis: &[
                "history",
                "history add command ?exec?",
                "history change newValue ?event?",
                "history clear",
                "history event ?event?",
                "history info ?count?",
                "history keep ?count?",
                "history nextid",
                "history redo ?event?",
            ],
            snippet: "",
            source: "Tcl stdlib history command",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        unsafe_command: true,
        ..CommandSpec::DEFAULT
    }
}
