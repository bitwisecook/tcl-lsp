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

//! `tk` command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "appname",
        arity: Arity::new(0, 1),
        detail: "Query or set the application name for send commands.",
        synopsis: "tk appname ?newName?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "busy",
        arity: Arity::at_least(1),
        detail: "Make a window appear busy (greyed out with a busy cursor).",
        synopsis: "tk busy subcommand ?arg ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "caret",
        arity: Arity::at_least(1),
        detail: "Query or set the caret (text cursor) position for accessibility.",
        synopsis: "tk caret window ?-x x? ?-y y? ?-height height?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "fontchooser",
        arity: Arity::at_least(1),
        detail: "Control the platform font selection dialogue.",
        synopsis: "tk fontchooser subcommand ?arg ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "inactive",
        arity: Arity::at_least(0),
        detail: "Query or reset the user inactivity timer in milliseconds.",
        synopsis: "tk inactive ?-displayof window? ?reset?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "scaling",
        arity: Arity::at_least(0),
        detail: "Query or set the number of pixels per point on the display.",
        synopsis: "tk scaling ?-displayof window? ?number?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "useinputmethods",
        arity: Arity::at_least(0),
        detail: "Query or set whether Tk should use XIM input methods.",
        synopsis: "tk useinputmethods ?-displayof window? ?boolean?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "windowingsystem",
        arity: Arity::exact(0),
        detail: "Return the windowing system in use: x11, win32, or aqua.",
        synopsis: "tk windowingsystem",
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

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tk subcommand ?arg ...?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tk",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Manipulate Tk internal state.",
            synopsis: &[
                "tk appname ?newName?",
                "tk busy subcommand ?arg ...?",
                "tk caret window ?-x x? ?-y y? ?-height height?",
                "tk fontchooser subcommand ?arg ...?",
                "tk inactive ?-displayof window? ?reset?",
                "tk scaling ?-displayof window? ?number?",
                "tk useinputmethods ?-displayof window? ?boolean?",
                "tk windowingsystem",
            ],
            snippet: "Provides access to miscellaneous Tk internal state and the windowing system.",
            source: "Tk man page tk.n",
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
