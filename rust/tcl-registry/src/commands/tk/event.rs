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

//! `event` command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        arity: Arity::at_least(2),
        detail: "Associate a virtual event with one or more physical event sequences.",
        synopsis: "event add <<virtual>> sequence ?sequence ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::at_least(1),
        detail: "Delete physical event sequences from a virtual event.",
        synopsis: "event delete <<virtual>> ?sequence sequence ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "generate",
        arity: Arity::at_least(2),
        detail: "Generate an event and arrange for it to be processed.",
        synopsis: "event generate window event ?option value ...?",
        options: OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::new(0, 1),
        detail: "Return information about virtual events.",
        synopsis: "event info ?<<virtual>>?",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-above",
        value: OptionValue::value("window"),
        detail: "Specifies the above field for the event (generate).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-borderwidth",
        value: OptionValue::value("size"),
        detail: "Specifies the border width for the event (generate).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-button",
        value: OptionValue::value("number"),
        detail: "Specifies the button number for a ButtonPress or ButtonRelease event.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-count",
        value: OptionValue::value("number"),
        detail: "Specifies the count field for the event.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-data",
        value: OptionValue::value("string"),
        detail: "Specifies user data for a virtual event.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-delta",
        value: OptionValue::value("number"),
        detail: "Specifies the delta field for a MouseWheel event.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-detail",
        value: OptionValue::value("detail"),
        detail: "Specifies the detail field for the event.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-focus",
        value: OptionValue::value("boolean"),
        detail: "Specifies the focus field for the event.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-height",
        value: OptionValue::value("size"),
        detail: "Specifies the height field for the event.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-keycode",
        value: OptionValue::value("number"),
        detail: "Specifies the keycode field for the event.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-keysym",
        value: OptionValue::value("name"),
        detail: "Specifies the keysym field for the event.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-mode",
        value: OptionValue::value("notify"),
        detail: "Specifies the mode field for the event.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-override",
        value: OptionValue::value("boolean"),
        detail: "Specifies the override-redirect field.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-place",
        value: OptionValue::value("where"),
        detail: "Specifies the place field for the event.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-root",
        value: OptionValue::value("window"),
        detail: "Specifies the root field for the event.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-rootx",
        value: OptionValue::value("coord"),
        detail: "Specifies the x-coordinate relative to the root window.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-rooty",
        value: OptionValue::value("coord"),
        detail: "Specifies the y-coordinate relative to the root window.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-sendevent",
        value: OptionValue::value("boolean"),
        detail: "Specifies the send-event field.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-serial",
        value: OptionValue::value("number"),
        detail: "Specifies the serial number field.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-state",
        value: OptionValue::value("state"),
        detail: "Specifies the state field for the event.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-subwindow",
        value: OptionValue::value("window"),
        detail: "Specifies the sub-window field.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-time",
        value: OptionValue::value("integer"),
        detail: "Specifies the time field for the event.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-warp",
        value: OptionValue::value("boolean"),
        detail: "Specifies whether the screen pointer should be warped.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-width",
        value: OptionValue::value("size"),
        detail: "Specifies the width field for the event.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-when",
        value: OptionValue::value("now|tail|head|mark"),
        detail: "Specifies when the event is processed.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-x",
        value: OptionValue::value("coord"),
        detail: "Specifies the x field for the event.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-y",
        value: OptionValue::value("coord"),
        detail: "Specifies the y field for the event.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "event option ?arg ...?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "event",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Generate, manage, and inspect virtual events.",
            synopsis: &[
                "event add <<virtual>> sequence ?sequence ...?",
                "event delete <<virtual>> ?sequence sequence ...?",
                "event generate window event ?option value ...?",
                "event info ?<<virtual>>?",
            ],
            snippet: "",
            source: "Tk man page event.n",
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
