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

//! `tk_messageBox` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-default",
        value: OptionValue::value("buttonName"),
        detail: "Name of the default button.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-detail",
        value: OptionValue::value("string"),
        detail: "Supplemental message text.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-icon",
        value: OptionValue::value("iconImage"),
        detail: "Icon to display (error, info, question, warning).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-message",
        value: OptionValue::value("string"),
        detail: "Main message text to display.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-parent",
        value: OptionValue::value("window"),
        detail: "Parent window for the dialogue.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-title",
        value: OptionValue::value("titleString"),
        detail: "Title string for the dialogue window.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-type",
        value: OptionValue::value("predefinedType"),
        detail: "Arrangement of buttons to display.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-command",
        value: OptionValue::command_prefix("prefix"),
        detail: "Command prefix invoked when the dialog closes; the clicked button name is appended (macOS).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tk_messageBox ?option value ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tk_messageBox",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Pop up a message window and wait for user response.",
            synopsis: &["tk_messageBox ?option value ...?"],
            snippet: "",
            source: "Tk man page tk_messageBox.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
