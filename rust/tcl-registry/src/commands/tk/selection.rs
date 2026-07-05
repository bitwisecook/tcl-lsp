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

//! `selection` command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "clear",
        arity: Arity::at_least(0),
        detail: "Clear the selection so that no window owns it.",
        synopsis: "selection clear ?-displayof window? ?-selection selection?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "get",
        arity: Arity::at_least(0),
        detail: "Retrieve the selection and return it as a string.",
        synopsis: "selection get ?-displayof window? ?-selection selection? ?-type type?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "handle",
        arity: Arity::at_least(2),
        detail: "Register a handler to provide the selection data.",
        synopsis: "selection handle ?-selection sel? ?-type type? ?-format fmt? window command",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "own",
        arity: Arity::at_least(0),
        detail: "Query or set the owner of the selection.",
        synopsis: "selection own ?-command command? ?-selection selection? window",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-displayof",
        takes_value: true,
        value_hint: "window",
        detail: "Specifies the display for the selection operation.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-selection",
        takes_value: true,
        value_hint: "selection",
        detail: "Specifies which named selection to operate on (default: PRIMARY).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-type",
        takes_value: true,
        value_hint: "type",
        detail: "Specifies the form in which the selection is to be returned.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-format",
        takes_value: true,
        value_hint: "format",
        detail: "Specifies the representation format for the selection data.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-command",
        takes_value: true,
        value_hint: "command",
        detail: "Specifies a Tcl script to run when the selection is claimed by another window.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "selection option ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "selection",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Manipulate the X selection.",
            synopsis: &[
                "selection clear ?-displayof window? ?-selection selection?",
                "selection get ?-displayof window? ?-selection selection? ?-type type?",
                "selection handle ?-selection selection? ?-type type? ?-format format? window command",
                "selection own ?-displayof window? ?-selection selection?",
                "selection own ?-command command? ?-selection selection? window",
            ],
            snippet: "",
            source: "Tk man page selection.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}
