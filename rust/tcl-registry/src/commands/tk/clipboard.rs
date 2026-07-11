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

//! `clipboard` command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "append",
        arity: Arity::at_least(1),
        detail: "Append data to the clipboard on the specified display.",
        synopsis: "clipboard append ?-displayof window? ?-format format? ?-type type? data",
        options: APPEND_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "clear",
        arity: Arity::at_least(0),
        detail: "Claim ownership of the clipboard and clear its contents.",
        synopsis: "clipboard clear ?-displayof window?",
        options: CLEAR_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "get",
        arity: Arity::at_least(0),
        detail: "Retrieve data from the clipboard on the specified display.",
        synopsis: "clipboard get ?-displayof window? ?-type type?",
        options: GET_OPTIONS,
        ..SubCommand::DEFAULT
    },
];

/// `append`: takes all three options — `-displayof`, `-format`, and
/// `-type` — per the Tk `clipboard.n` manual page.
const APPEND_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-displayof",
        value: OptionValue::value("window"),
        detail: "Specifies the display for the clipboard operation.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-format",
        value: OptionValue::value("format"),
        detail: "Specifies the representation format for the data (append).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-type",
        value: OptionValue::value("type"),
        detail: "Specifies the form in which the selection is to be returned.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

/// `clear`: only `-displayof` — no `-format`/`-type` per the manual page.
const CLEAR_OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-displayof",
    value: OptionValue::value("window"),
    detail: "Specifies the display for the clipboard operation.",
    dialects: None,
    aliases: &[],
    min_version: None,
}];

/// `get`: `-displayof` and `-type` — no `-format` (that only applies to
/// `append`) per the manual page.
const GET_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-displayof",
        value: OptionValue::value("window"),
        detail: "Specifies the display for the clipboard operation.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-type",
        value: OptionValue::value("type"),
        detail: "Specifies the form in which the selection is to be returned.",
        dialects: None,
        aliases: &[],
        min_version: None,
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
        value: OptionValue::value("window"),
        detail: "Specifies the display for the clipboard operation.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-format",
        value: OptionValue::value("format"),
        detail: "Specifies the representation format for the data (append).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-type",
        value: OptionValue::value("type"),
        detail: "Specifies the form in which the selection is to be returned.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "clipboard option ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "clipboard",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Manipulate the Tk clipboard.",
            synopsis: &[
                "clipboard append ?-displayof window? ?-format format? ?-type type? data",
                "clipboard clear ?-displayof window?",
                "clipboard get ?-displayof window? ?-type type?",
            ],
            snippet: "",
            source: "Tk man page clipboard.n",
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
