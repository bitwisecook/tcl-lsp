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

//! `tkwait` command.
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "variable",
        arity: Arity::exact(1),
        detail: "Wait until the global variable name is set.",
        synopsis: "tkwait variable name",
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "visibility",
        arity: Arity::exact(1),
        detail: "Wait until the visibility state of window changes.",
        synopsis: "tkwait visibility window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "window",
        arity: Arity::exact(1),
        detail: "Wait until window is destroyed.",
        synopsis: "tkwait window window",
        ..SubCommand::DEFAULT
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tkwait variable|visibility|window name",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tkwait",
        // `tkwait variable` reads a global variable by name through the
        // frame hash bucket — hence the FRAME_HASH_BUILTIN trait the
        // var-escape slot resolver keys on.
        traits: Traits::FRAME_HASH_BUILTIN,
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::exact(2),
        subcommands: SUBCOMMANDS,
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Wait for an event (variable set, window visibility, or destruction).",
            synopsis: &[
                "tkwait variable name",
                "tkwait visibility window",
                "tkwait window window",
            ],
            snippet: "",
            source: "Tk man page tkwait.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
