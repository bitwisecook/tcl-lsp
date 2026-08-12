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

/// Dynamic arg-role resolver for `selection handle`.
///
/// `selection handle ?-selection s? ?-type t? ?-format f? window command`
/// — the trailing `command` is a command *prefix* that Tk invokes with
/// two numbers (offset and maxChars) appended to supply selection data,
/// so it carries [`ArgRole::CommandPrefix`] (its first word is a command
/// reference), not a script body.  It is always the last argument; the
/// leading option/value pairs and `window` precede it.  Args here are
/// those *after* the `handle` subcommand word.
fn selection_handle_command_prefixes(args: CommandPrefixArguments<'_>) -> Vec<(u8, AppendedArity)> {
    match u8::try_from(args.len()) {
        // window + command are both required (arity at_least(2)), so the
        // command prefix is the final argument.  Tk invokes it with `offset
        // maxChars` appended → 2 args (`Exactly(2)`).
        Ok(n) if n >= 2 => vec![(n - 1, AppendedArity::Exactly(2))],
        _ => Vec::new(),
    }
}

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "clear",
        arity: Arity::at_least(0),
        detail: "Clear the selection so that no window owns it.",
        synopsis: "selection clear ?-displayof window? ?-selection selection?",
        options: CLEAR_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "get",
        arity: Arity::at_least(0),
        detail: "Retrieve the selection and return it as a string.",
        synopsis: "selection get ?-displayof window? ?-selection selection? ?-type type?",
        options: GET_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "handle",
        arity: Arity::at_least(2),
        detail: "Register a handler to provide the selection data.",
        synopsis: "selection handle ?-selection sel? ?-type type? ?-format fmt? window command",
        command_prefix_resolver: Some(selection_handle_command_prefixes),
        options: HANDLE_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "own",
        arity: Arity::at_least(0),
        detail: "Query or set the owner of the selection.",
        synopsis: "selection own ?-command command? ?-selection selection? window",
        options: OWN_OPTIONS,
        ..SubCommand::DEFAULT
    },
];

/// `clear`: display and selection-name selectors only.
const CLEAR_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-displayof",
        value: OptionValue::value("window"),
        detail: "Specifies the display for the selection operation.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-selection",
        value: OptionValue::value("selection"),
        detail: "Specifies which named selection to operate on (default: PRIMARY).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

/// `get`: `CLEAR_OPTIONS` plus `-type` for the desired return format.
const GET_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-displayof",
        value: OptionValue::value("window"),
        detail: "Specifies the display for the selection operation.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-selection",
        value: OptionValue::value("selection"),
        detail: "Specifies which named selection to operate on (default: PRIMARY).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-type",
        value: OptionValue::value("type"),
        detail: "Specifies the form in which the selection is to be returned.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

/// `handle`: no `-displayof` (the window argument implies the display) —
/// `-selection`, `-type`, and `-format` for the registered handler.
const HANDLE_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-selection",
        value: OptionValue::value("selection"),
        detail: "Specifies which named selection to operate on (default: PRIMARY).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-type",
        value: OptionValue::value("type"),
        detail: "Specifies the form in which the selection is to be returned.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-format",
        value: OptionValue::value("format"),
        detail: "Specifies the representation format for the selection data.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

/// `own`: `-displayof`/`-selection` for the query form, plus `-command` for
/// the claim form (window becomes the new owner and runs the script on loss).
const OWN_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-displayof",
        value: OptionValue::value("window"),
        detail: "Specifies the display for the selection operation.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-selection",
        value: OptionValue::value("selection"),
        detail: "Specifies which named selection to operate on (default: PRIMARY).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-command",
        value: OptionValue::script(),
        detail: "Specifies a Tcl script to run when the selection is claimed by another window.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-displayof",
        value: OptionValue::value("window"),
        detail: "Specifies the display for the selection operation.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-selection",
        value: OptionValue::value("selection"),
        detail: "Specifies which named selection to operate on (default: PRIMARY).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-type",
        value: OptionValue::value("type"),
        detail: "Specifies the form in which the selection is to be returned.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-format",
        value: OptionValue::value("format"),
        detail: "Specifies the representation format for the selection data.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-command",
        value: OptionValue::script(),
        detail: "Specifies a Tcl script to run when the selection is claimed by another window.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "selection option ?arg ...?",
    dialects: None,
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
