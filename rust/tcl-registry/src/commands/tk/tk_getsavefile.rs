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

//! `tk_getSaveFile` command.
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
        name: "-confirmoverwrite",
        value: OptionValue::boolean(),
        detail: "Prompt for confirmation if the file already exists.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-defaultextension",
        value: OptionValue::value("extension"),
        detail: "Default extension to append if the user does not type one.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-filetypes",
        value: OptionValue::value("filePatternList"),
        detail: "List of file type patterns to display in the filter.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-initialdir",
        value: OptionValue::value("dirName"),
        detail: "Initial directory to display.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-initialfile",
        value: OptionValue::value("fileName"),
        detail: "Initial file name to populate in the dialogue.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-parent",
        value: OptionValue::value("window"),
        detail: "Parent window for the dialogue.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-title",
        value: OptionValue::value("titleString"),
        detail: "Title string for the dialogue window.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-typevariable",
        value: OptionValue::var_name(),
        detail: "Variable to store the selected file type.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-command",
        value: OptionValue::command_prefix("prefix"),
        detail: "Command prefix invoked when the dialog closes; the chosen file is appended (macOS).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-message",
        value: OptionValue::value("string"),
        detail: "Message displayed in the dialog (macOS).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tk_getSaveFile ?option value ...?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tk_getSaveFile",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Pop up a dialogue for the user to select a file to save.",
            synopsis: &["tk_getSaveFile ?option value ...?"],
            snippet: "",
            source: "Tk man page tk_getSaveFile.n",
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
