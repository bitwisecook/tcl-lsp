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

//! `ttk::style` command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "configure",
        arity: Arity::at_least(1),
        detail: "Set or query style options.",
        synopsis: "ttk::style configure style ?-option ?value ...??",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "element",
        arity: Arity::at_least(1),
        detail: "Manage style elements.",
        synopsis: "ttk::style element subcommand ?args?",
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "create",
                    detail: "Create a new element.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "names",
                    detail: "Return a list of all registered element names.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "options",
                    detail: "Return the list of options for an element.",
                    ..ArgValue::DEFAULT
                },
            ],
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "layout",
        arity: Arity::new(1, 2),
        detail: "Define or query the layout of a style.",
        synopsis: "ttk::style layout style ?layoutSpec?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lookup",
        arity: Arity::new(2, 4),
        detail: "Look up a style option value.",
        synopsis: "ttk::style lookup style -option ?state? ?default?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "map",
        arity: Arity::at_least(1),
        detail: "Set dynamic (state-dependent) style options.",
        synopsis: "ttk::style map style ?-option {statespec value ...} ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "theme",
        arity: Arity::at_least(1),
        detail: "Manage and query themes.",
        synopsis: "ttk::style theme subcommand ?args?",
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "create",
                    detail: "Create a new theme.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "names",
                    detail: "Return a list of available theme names.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "settings",
                    detail: "Evaluate a script in the context of a theme.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "use",
                    detail: "Set the current theme.",
                    ..ArgValue::DEFAULT
                },
            ],
        )],
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "ttk::style subcommand ?arg ...?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::style",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Manipulate ttk styles and themes.",
            synopsis: &["ttk::style subcommand ?arg ...?"],
            snippet: "",
            source: "Tk man page ttk_style.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        lifecycle: Lifecycle::introduced_in("8.5"),
        warn_missing_import: false,
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}
