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

//! `grab` command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "current",
        arity: Arity::new(0, 1),
        detail: "Return the path name of the current grab window, if any.",
        synopsis: "grab current ?window?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "release",
        arity: Arity::exact(1),
        detail: "Release the grab on the window.",
        synopsis: "grab release window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "set",
        arity: Arity::new(1, 2),
        detail: "Set a grab on the window, optionally global.",
        synopsis: "grab set ?-global? window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "status",
        arity: Arity::exact(1),
        detail: "Return the grab status of the window (none, local, or global).",
        synopsis: "grab status window",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-global",
    value: OptionValue::flag(),
    detail: "Make the grab global (applies to all displays).",
    dialects: None,
    aliases: &[],
    lifecycle: Lifecycle::UNSPECIFIED,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "grab option ?arg ...?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "grab",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Confine pointer and keyboard events to a window sub-tree.",
            synopsis: &[
                "grab ?-global? window",
                "grab current ?window?",
                "grab release window",
                "grab set ?-global? window",
                "grab status window",
            ],
            snippet: "",
            source: "Tk man page grab.n",
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
